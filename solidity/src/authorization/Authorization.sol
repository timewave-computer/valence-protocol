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
 * on specific contracts through the processor.
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
     * @dev This event is emitted when a new authorization is granted to a user for a specific contract and function
     * @dev Only used for Standard authorizations
     * @param user The address of the user that was granted authorization. If address(0) is used, then it's permissionless
     * @param contractAddress The address of the contract the user is authorized to interact with
     * @param callHash The hash of the function call that the user is authorized to execute
     */
    event AuthorizationAdded(address indexed user, address indexed contractAddress, bytes32 indexed callHash);
    /**
     * @notice Event emitted when an authorization is removed
     * @dev This event is emitted when an authorization is revoked from a user for a specific contract and function
     * @dev Only used for Standard authorizations
     * @param user The address of the user that had authorization revoked. If address(0) is used, then it's permissionless
     * @param contractAddress The address of the contract the user had authorization for
     * @param callHash The hash of the function call that the user had authorization to execute
     */
    event AuthorizationRemoved(address indexed user, address indexed contractAddress, bytes32 indexed callHash);

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
     * @notice Multi-dimensional mapping for granular authorization control
     * @dev Maps from user address -> contract address -> function signature hash -> boolean
     * If address(0) is used as the user address, it indicates permissionless access
     * Represents the operations a specific address can execute on a specific contract
     */
    mapping(address => mapping(address => mapping(bytes32 => bool))) public authorizations;

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
     * @notice Grants authorization for multiple users to call specific functions on specific contracts
     * @dev Can only be called by the owner
     * @param _users Array of addresses being granted authorization, if address(0) is used, then it's permissionless
     * @param _contracts Array of contract addresses the users are authorized to interact with
     * @param _calls Array of function call data (used to generate hashes for authorization checking)
     */
    function addStandardAuthorizations(address[] memory _users, address[] memory _contracts, bytes[] memory _calls)
        external
        onlyOwner
    {
        // Check that the arrays are the same length
        // We are allowing adding multiple authorizations at once for gas optimization
        // The arrays must be the same length because for each user we have a contract and a call
        require(_users.length == _contracts.length && _contracts.length == _calls.length, "Array lengths must match");

        for (uint256 i = 0; i < _users.length; i++) {
            bytes32 callHash = keccak256(_calls[i]);
            authorizations[_users[i]][_contracts[i]][callHash] = true;
            emit AuthorizationAdded(_users[i], _contracts[i], callHash);
        }
    }

    /**
     * @notice Revokes authorization for multiple users to call specific functions on specific contracts
     * @dev Can only be called by the owner
     * @param _users Array of addresses having authorization revoked
     * @param _contracts Array of contract addresses the authorizations apply to
     * @param _calls Array of function call data (used to generate the hashes for lookup)
     */
    function removeStandardAuthorizations(address[] memory _users, address[] memory _contracts, bytes[] memory _calls)
        external
        onlyOwner
    {
        require(_users.length == _contracts.length && _contracts.length == _calls.length, "Array lengths must match");

        for (uint256 i = 0; i < _users.length; i++) {
            address user = _users[i];
            address contractAddress = _contracts[i];
            bytes32 callHash = keccak256(_calls[i]);
            delete authorizations[user][contractAddress][callHash];
            emit AuthorizationRemoved(user, contractAddress, callHash);
        }
    }

    /**
     * @notice Main function to send messages to the processor after authorization checks
     * @dev Delegates to specialized helper functions based on message type
     * @param _message Encoded processor message to be executed
     */
    function sendProcessorMessage(bytes calldata _message) external nonReentrant {
        // Make a copy of the message to apply modifications
        bytes memory message = _message;

        // Decode the message to check authorization and apply modifications
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage = ProcessorMessageDecoder.decode(message);

        // Process message based on type
        if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            message = _handleSendMsgsMessage(decodedMessage);
        } else if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs) {
            message = _handleInsertMsgsMessage(decodedMessage);
        } else {
            _requireAdminAccess();
        }

        // Forward the validated and modified message to the processor
        processor.execute(message);

        // Increment the execution ID for the next message
        executionId++;
    }

    /**
     * @notice Handle InsertMsgs type messages
     * @dev Requires admin access and sets execution ID
     * @param decodedMessage The decoded processor message
     * @return The modified encoded message
     */
    function _handleInsertMsgsMessage(IProcessorMessageTypes.ProcessorMessage memory decodedMessage)
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
     * @notice Handle SendMsgs type messages
     * @dev Checks function-level authorizations and modifies priority and execution ID
     * @param decodedMessage The decoded processor message
     * @return The modified encoded message
     */
    function _handleSendMsgsMessage(IProcessorMessageTypes.ProcessorMessage memory decodedMessage)
        private
        view
        returns (bytes memory)
    {
        // Decode the SendMsgs message
        IProcessorMessageTypes.SendMsgs memory sendMsgs =
            abi.decode(decodedMessage.message, (IProcessorMessageTypes.SendMsgs));

        // Verify authorizations based on subroutine type
        if (sendMsgs.subroutine.subroutineType == IProcessorMessageTypes.SubroutineType.Atomic) {
            _verifyAtomicSubroutineAuthorization(sendMsgs);
        } else {
            _verifyNonAtomicSubroutineAuthorization(sendMsgs);
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
     * @notice Verify authorization for atomic subroutine messages
     * @dev Checks that each function call is authorized for the sender
     * @param sendMsgs The SendMsgs message containing the atomic subroutine
     */
    function _verifyAtomicSubroutineAuthorization(IProcessorMessageTypes.SendMsgs memory sendMsgs) private view {
        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

        // Verify message and function array lengths match
        if (atomicSubroutine.functions.length > 0 && atomicSubroutine.functions.length != sendMsgs.messages.length) {
            revert("Subroutine functions length does not match messages length");
        }

        // Check authorization for each function in the atomic subroutine
        for (uint256 i = 0; i < atomicSubroutine.functions.length; i++) {
            if (
                !_checkAddressIsAuthorized(
                    msg.sender, atomicSubroutine.functions[i].contractAddress, sendMsgs.messages[i]
                )
            ) {
                revert("Unauthorized access");
            }
        }
    }

    /**
     * @notice Verify authorization for non-atomic subroutine messages
     * @dev Checks that each function call is authorized for the sender
     * @param sendMsgs The SendMsgs message containing the non-atomic subroutine
     */
    function _verifyNonAtomicSubroutineAuthorization(IProcessorMessageTypes.SendMsgs memory sendMsgs) private view {
        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

        // Verify message and function array lengths match
        if (
            nonAtomicSubroutine.functions.length > 0 && nonAtomicSubroutine.functions.length != sendMsgs.messages.length
        ) {
            revert("Subroutine functions length does not match messages length");
        }

        // Check authorization for each function in the non-atomic subroutine
        for (uint256 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
            if (
                !_checkAddressIsAuthorized(
                    msg.sender, nonAtomicSubroutine.functions[i].contractAddress, sendMsgs.messages[i]
                )
            ) {
                revert("Unauthorized access");
            }
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
     * @notice Checks if an address is authorized to execute a specific call on a specific contract
     * @dev Uses the authorizations mapping to perform the check
     * @param _address Address to check authorization for
     * @param _contract Address of the contract being called
     * @param _call Function call data (used to generate the hash for lookup)
     * @return bool True if the address is authorized, false otherwise
     */
    function _checkAddressIsAuthorized(address _address, address _contract, bytes memory _call)
        internal
        view
        returns (bool)
    {
        // Check if the address is authorized to call the contract with the given call
        if (authorizations[_address][_contract][keccak256(_call)]) {
            return true;
        } else if (authorizations[address(0)][_contract][keccak256(_call)]) {
            // If address(0) is used, it indicates permissionless access
            return true;
        } else {
            return false;
        }
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
