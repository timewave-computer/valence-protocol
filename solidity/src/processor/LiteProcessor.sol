// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";
import {ProcessorMessageDecoder} from "./libs/ProcessorMessageDecoder.sol";
import {IProcessorMessageTypes} from "./interfaces/IProcessorMessageTypes.sol";

/**
 * @title LiteProcessor
 * @notice A lightweight processor for handling cross-chain messages with atomic and non-atomic execution
 * @dev Implements IMessageRecipient for Hyperlane message handling
 */
contract LiteProcessor is IMessageRecipient {
    // ============ State Variables ============

    /**
     * @notice The authorized contract that can send messages from the main domain
     * @dev Stored as bytes32 to handle cross-chain address representation
     */
    bytes32 public immutable authorizationContract;

    /**
     * @notice The only address allowed to deliver messages to this processor
     * @dev This should be the Hyperlane mailbox contract
     */
    address public immutable mailbox;

    /**
     * @notice Indicates if the processor is currently paused
     */
    bool public paused;

    // ============ Events ============

    /**
     * @notice Emitted when a message is received by the processor
     * @param origin The domain ID where the message originated
     * @param sender The sender's address in bytes32 format
     * @param body The raw message bytes
     */
    event MessageReceived(uint32 indexed origin, bytes32 indexed sender, bytes body);

    /**
     * @notice Event emitted after a subroutine is processed
     * @dev This event provides complete information about the execution result,
     *      allowing external systems to track and respond to subroutine execution outcomes
     * @param isAtomic Whether this was an atomic subroutine (true) or non-atomic (false)
     * @param succeeded Overall execution success status
     *        - For atomic: true if all functions succeeded, false if any failed
     *        - For non-atomic: true if all executed, false if stopped due to failure
     * @param executedCount Number of successfully executed functions
     *        - For atomic: Will be 0 if failed, total count if succeeded
     *        - For non-atomic: Number of functions that executed before any failure
     * @param errorData Raw error data from the failed execution
     *        - Empty bytes if execution succeeded
     *        - Contains the error data from the first failed function if execution failed
     *        - Format depends on how the called contract reverted (custom error, string, etc.)
     */
    event SubroutineProcessed(bool isAtomic, bool succeeded, uint256 executedCount, bytes errorData);

    /**
     * @notice Emitted when the processor is paused
     */
    event ProcessorPaused();

    /**
     * @notice Emitted when the processor is resumed
     */
    event ProcessorResumed();

    /**
     * @notice Emitted when a SendMsgs operation is processed
     */
    event ProcessedSendMsgsOperation();

    // ============ Custom Errors ============

    error UnauthorizedAccessError();
    error NotAuthorizationContractError();
    error InvalidAddressError();
    error ProcessorPausedError();
    error UnsupportedOperationError();

    // ============ Constructor ============

    /**
     * @notice Initializes the LiteProcessor
     * @param _authorizationContract The authorized contract address in bytes32
     * @param _mailbox The Hyperlane mailbox address
     */
    constructor(bytes32 _authorizationContract, address _mailbox) {
        if (_mailbox == address(0)) {
            revert InvalidAddressError();
        }

        authorizationContract = _authorizationContract;
        mailbox = _mailbox;
    }

    // ============ External Functions ============

    /**
     * @notice Handles incoming messages from the Hyperlane mailbox
     * @param _origin The origin domain ID
     * @param _sender The sender's address in bytes32
     * @param _body The message payload
     */
    function handle(uint32 _origin, bytes32 _sender, bytes calldata _body) external payable override {
        // Verify sender is authorized mailbox
        if (msg.sender != mailbox) {
            revert UnauthorizedAccessError();
        }

        // Verify message is from authorized contract
        if (_sender != authorizationContract) {
            revert NotAuthorizationContractError();
        }

        // Emit reception before processing
        emit MessageReceived(_origin, _sender, _body);

        // Decode and route message to appropriate handler
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage = ProcessorMessageDecoder.decode(_body);
        _handleMessageType(decodedMessage);
    }

    // ============ Internal Functions ============

    /**
     * @notice Routes the message to appropriate handler based on message type
     * @param decodedMessage The decoded processor message
     */
    function _handleMessageType(IProcessorMessageTypes.ProcessorMessage memory decodedMessage) internal {
        if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.Pause) {
            _handlePause();
        } else if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.Resume) {
            _handleResume();
        } else if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            _handleSendMsgs(decodedMessage);
            emit ProcessedSendMsgsOperation();
        } else {
            revert UnsupportedOperationError();
        }
    }

    /**
     * @notice Result of a subroutine execution
     * @param succeeded Whether all functions executed successfully
     * @param executedCount Number of successfully executed functions before failure or completion. For atomic subroutines, this will be the total count if all succeeded
     * @param errorData The error data from the last failed function, empty if all succeeded
     */
    struct SubroutineResult {
        bool succeeded;
        uint256 executedCount;
        bytes errorData;
    }

    /**
     * @notice Handles pause messages
     */
    function _handlePause() internal {
        paused = true;
        emit ProcessorPaused();
    }

    /**
     * @notice Handles resume messages
     */
    function _handleResume() internal {
        paused = false;
        emit ProcessorResumed();
    }

    /**
     * @notice Processes SendMsgs operations based on subroutine type
     * @dev Decodes and routes to appropriate subroutine handler
     * @param decodedMessage The decoded processor message
     */
    function _handleSendMsgs(IProcessorMessageTypes.ProcessorMessage memory decodedMessage) internal {
        // Check if the processor is paused
        if (paused) {
            revert ProcessorPausedError();
        }

        IProcessorMessageTypes.SendMsgs memory sendMsgs =
            abi.decode(decodedMessage.message, (IProcessorMessageTypes.SendMsgs));

        if (sendMsgs.subroutine.subroutineType == IProcessorMessageTypes.SubroutineType.Atomic) {
            SubroutineResult memory result = _handleAtomicSubroutine(sendMsgs);
            emit SubroutineProcessed(true, result.succeeded, result.executedCount, result.errorData);
        } else {
            SubroutineResult memory result = _handleNonAtomicSubroutine(sendMsgs);
            emit SubroutineProcessed(false, result.succeeded, result.executedCount, result.errorData);
        }
    }

    /**
     * @notice Executes all functions in an atomic subroutine
     * @dev Either all functions succeed or no state changes are committed
     * @param sendMsgs The SendMsgs operation containing the atomic subroutine
     * @return result Contains execution success status, executed function count (all or 0), and error data if any failed
     */
    function _handleAtomicSubroutine(IProcessorMessageTypes.SendMsgs memory sendMsgs)
        internal
        returns (SubroutineResult memory)
    {
        try this._executeAtomicSubroutine(sendMsgs) returns (uint256 totalExecuted) {
            return SubroutineResult({succeeded: true, executedCount: totalExecuted, errorData: ""});
        } catch (bytes memory err) {
            return SubroutineResult({succeeded: false, executedCount: 0, errorData: err});
        }
    }

    /**
     * @notice Executes functions in a non-atomic subroutine until one fails
     * @dev Processes functions one by one, stopping at first failure
     * @param sendMsgs The SendMsgs operation containing the non-atomic subroutine
     * @return result Contains execution count and error data if any failed
     */
    function _handleNonAtomicSubroutine(IProcessorMessageTypes.SendMsgs memory sendMsgs)
        internal
        returns (SubroutineResult memory)
    {
        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

        uint256 executedCount = 0;
        bytes memory errorData;
        bool succeeded = true;

        // Execute each function until one fails
        for (uint256 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
            (bool success, bytes memory err) =
                nonAtomicSubroutine.functions[i].contractAddress.call(sendMsgs.messages[i]);

            if (success) {
                executedCount++;
            } else {
                succeeded = false;
                errorData = err;
                break;
            }
        }

        return SubroutineResult({succeeded: succeeded, executedCount: executedCount, errorData: errorData});
    }

    /**
     * @notice External function that executes the atomic subroutine and reverts if any fail
     * @dev External to allow try-catch pattern for atomicity
     * @param sendMsgs The SendMsgs operation containing the atomic subroutine
     * @return totalExecuted Number of functions executed
     */
    function _executeAtomicSubroutine(IProcessorMessageTypes.SendMsgs memory sendMsgs) external returns (uint256) {
        // Only allow calls from the contract itself, need this extra protection to prevent external access
        // This is necessary because the function is external and can be called by anyone
        // It's external to allow try-catch pattern for atomicity
        if (msg.sender != address(this)) {
            revert UnauthorizedAccessError();
        }

        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

        for (uint256 i = 0; i < atomicSubroutine.functions.length; i++) {
            /**
             * @notice Executes a contract call and forwards any error if the call fails
             * @dev When a contract call fails, Solidity captures the revert data (error)
             *      in a bytes array with a 32-byte length prefix. To correctly propagate
             *      the original error, we need to:
             *      1. Capture both success status and error data from the call
             *      2. If call failed, use assembly to revert with the original error:
             *         - Skip the 32-byte length prefix in memory (add(err, 32))
             *         - Use the length value at the start of err (mload(err))
             *         - Revert with exactly the original error data
             */
            (bool success, bytes memory err) = atomicSubroutine.functions[i].contractAddress.call(sendMsgs.messages[i]);
            if (!success) {
                // Forward the original error data
                assembly {
                    revert(add(err, 32), mload(err))
                }
            }
        }

        // Return the total number of executed functions
        return atomicSubroutine.functions.length;
    }
}
