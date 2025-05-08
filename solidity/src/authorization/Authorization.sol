// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ProcessorBase} from "../processor/ProcessorBase.sol";
import {IProcessorMessageTypes} from "../processor/interfaces/IProcessorMessageTypes.sol";
import {ProcessorMessageDecoder} from "../processor/libs/ProcessorMessageDecoder.sol";
import {ICallback} from "../processor/interfaces/ICallback.sol";
import {IProcessor} from "../processor/interfaces/IProcessor.sol";

/**
 * @title Authorization
 * @dev This contract manages authorizations for interactions with a processor contract.
 * It provides mechanisms for both standard address-based authorizations and ZK proof-based authorizations.
 * @notice The Authorization contract acts as a middleware for managing access control
 * to the Processor contract. It controls which addresses can call specific functions
 * on specific contracts through the processor.
 */
contract Authorization is Ownable, ICallback {
    // Address of the processor that we will forward batches to
    ProcessorBase public processor;

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
     *     @dev This mapping stores the callback data for each execution ID
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
     * @notice Address of the verifier contract used for zero-knowledge proof verification
     * @dev If zero-knowledge proofs are not being used, this can be set to address(0)
     */
    address public verifier;

    /**
     * @notice Sets up the Authorization contract with initial configuration
     * @dev Initializes the contract with owner, processor, and optional verifier
     * @param _owner Address that will be set as the owner of this contract
     * @param _processor Address of the processor contract that will execute messages
     * @param _verifier Address of the ZK verifier contract (can be address(0) if not using ZK proofs)
     * @param _storeCallbacks Boolean indicating whether to store callbacks or just emitting events
     */
    constructor(address _owner, address _processor, address _verifier, bool _storeCallbacks) Ownable(_owner) {
        if (_processor == address(0)) {
            revert("Processor cannot be zero address");
        }
        processor = ProcessorBase(_processor);
        verifier = _verifier;
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
     * @notice Updates the ZK verifier contract address
     * @dev Can only be called by the owner
     * @param _verifier New verifier contract address
     */
    function updateVerifier(address _verifier) external onlyOwner {
        verifier = _verifier;
    }

    /**
     * @notice Adds an address to the list of admin addresses
     * @dev Can only be called by the owner
     * @param _admin Address to be granted admin privileges
     */
    function addAdminAddress(address _admin) external onlyOwner {
        adminAddresses[_admin] = true;
    }

    /**
     * @notice Removes an address from the list of admin addresses
     * @dev Can only be called by the owner
     * @param _admin Address to have admin privileges revoked
     */
    function removeAdminAddress(address _admin) external onlyOwner {
        delete adminAddresses[_admin];
    }

    /**
     * @notice Grants authorization for a user to call a specific function on a specific contract
     * @dev Can only be called by the owner
     * @param _user Address of the user being granted authorization, if address(0) is used, then it's permissionless
     * @param _contract Address of the contract the user is authorized to interact with
     * @param _call Function call data (used to generate a hash for authorization checking)
     */
    function addStandardAuthorization(address _user, address _contract, bytes memory _call) external onlyOwner {
        authorizations[_user][_contract][keccak256(_call)] = true;
    }

    /**
     * @notice Revokes authorization for a user to call a specific function on a specific contract
     * @dev Can only be called by the owner
     * @param _user Address of the user having authorization revoked
     * @param _contract Address of the contract the authorization applies to
     * @param call Function call data (used to generate the hash for lookup)
     */
    function removeStandardAuthorization(address _user, address _contract, bytes memory call) external onlyOwner {
        delete authorizations[_user][_contract][keccak256(call)];
    }

    /**
     * @notice Main function to send messages to the processor after authorization checks
     * @dev Handles various message types differently:
     *      - For SendMsgs: Checks authorization for each function call
     *      - For InsertMsgs: Requires admin access and sets execution ID
     *      - For other types: Requires admin access
     * @param _message Encoded processor message to be executed
     */
    function sendProcessorMessage(bytes calldata _message) external {
        // Make a copy of the message to apply modifications
        bytes memory message = _message;

        // Decode the message to check authorization and apply modifications
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage = ProcessorMessageDecoder.decode(message);

        // Handle different message types with different authorization requirements
        if (decodedMessage.messageType != IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            // For non-SendMsgs messages, only admin addresses are authorized
            if (!adminAddresses[msg.sender]) {
                revert("Unauthorized access");
            }

            // Special handling for InsertMsgs to set execution ID
            if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs) {
                IProcessorMessageTypes.InsertMsgs memory insertMsgs =
                    abi.decode(decodedMessage.message, (IProcessorMessageTypes.InsertMsgs));

                // Set the execution ID of the message
                insertMsgs.executionId = executionId;

                // Encode the message back after modification
                decodedMessage.message = abi.encode(insertMsgs);

                // Encode the processor message back to bytes
                message = abi.encode(decodedMessage);
            }
        } else {
            // For SendMsgs, check function-level authorizations

            // Decode the SendMsgs message
            IProcessorMessageTypes.SendMsgs memory sendMsgs =
                abi.decode(decodedMessage.message, (IProcessorMessageTypes.SendMsgs));

            // Handle different subroutine types (Atomic vs NonAtomic)
            if (sendMsgs.subroutine.subroutineType == IProcessorMessageTypes.SubroutineType.Atomic) {
                IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
                    abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

                // Verify message and function array lengths match
                if (
                    atomicSubroutine.functions.length > 0
                        && atomicSubroutine.functions.length != sendMsgs.messages.length
                ) {
                    revert("Subroutine functions length does not match messages length");
                }

                // Check authorization for each function in the atomic subroutine
                for (uint256 i = 0; i < atomicSubroutine.functions.length; i++) {
                    if (
                        !_checkUserIsAuthorized(
                            msg.sender, atomicSubroutine.functions[i].contractAddress, sendMsgs.messages[i]
                        )
                    ) {
                        revert("Unauthorized access");
                    }
                }
            } else {
                // Handle NonAtomic subroutine
                IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
                    abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

                // Verify message and function array lengths match
                if (
                    nonAtomicSubroutine.functions.length > 0
                        && nonAtomicSubroutine.functions.length != sendMsgs.messages.length
                ) {
                    revert("Subroutine functions length does not match messages length");
                }

                // Check authorization for each function in the non-atomic subroutine
                for (uint256 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
                    if (
                        !_checkUserIsAuthorized(
                            msg.sender, nonAtomicSubroutine.functions[i].contractAddress, sendMsgs.messages[i]
                        )
                    ) {
                        revert("Unauthorized access");
                    }
                }
            }

            // Force the priority to Medium for all SendMsgs
            sendMsgs.priority = IProcessorMessageTypes.Priority.Medium;

            // Set the execution ID of the message
            sendMsgs.executionId = executionId;

            // Encode the message back after modifications
            decodedMessage.message = abi.encode(sendMsgs);

            // Encode the processor message back to bytes
            message = abi.encode(decodedMessage);
        }

        // Increment the execution ID for the next message
        executionId++;

        // Forward the validated and modified message to the processor
        processor.execute(message);
    }

    /**
     * @notice Checks if a user is authorized to execute a specific call on a specific contract
     * @dev Uses the authorizations mapping to perform the check
     * @param _user Address of the user to check authorization for
     * @param _contract Address of the contract being called
     * @param _call Function call data (used to generate the hash for lookup)
     * @return bool True if the user is authorized, false otherwise
     */
    function _checkUserIsAuthorized(address _user, address _contract, bytes memory _call)
        internal
        view
        returns (bool)
    {
        // Check if the user is authorized to call the contract with the given call
        if (authorizations[_user][_contract][keccak256(_call)]) {
            return true;
        } else if (authorizations[address(0)][_contract][keccak256(_call)]) {
            // If address(0) is used, it indicates permissionless access
            return true;
        } else {
            return false;
        }
    }

    function handleCallback(bytes memory callbackData) external override {
        // Check that the sender is the processor
        if (msg.sender != address(processor)) {
            revert("Only processor can send callbacks");
        }
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
