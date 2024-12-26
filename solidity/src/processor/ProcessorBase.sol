// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {IProcessorMessageTypes} from "./interfaces/IProcessorMessageTypes.sol";
import {IProcessor} from "./interfaces/IProcessor.sol";
import {ProcessorEvents} from "./libs/ProcessorEvents.sol";

abstract contract ProcessorBase {
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

    /**
     * @notice Initializes the state variables
     * @param _authorizationContract The authorized contract address in bytes32
     * @param _mailbox The Hyperlane mailbox address
     */
    constructor(bytes32 _authorizationContract, address _mailbox) {
        if (_mailbox == address(0)) {
            revert ProcessorErrors.InvalidAddressError();
        }
        authorizationContract = _authorizationContract;
        mailbox = _mailbox;
    }

    /**
     * @notice Handles pause messages
     */
    function _handlePause() internal {
        paused = true;
        emit ProcessorEvents.ProcessorPaused();
    }

    /**
     * @notice Handles resume messages
     */
    function _handleResume() internal {
        paused = false;
        emit ProcessorEvents.ProcessorResumed();
    }

    /**
     * @notice Executes all functions in an atomic subroutine
     * @dev Either all functions succeed or no state changes are committed
     * @param atomicSubroutine The atomic subroutine to execute
     * @param messages The messages to be sent for each contract call
     * @return result Contains execution success status, executed function count (all or 0), and error data if any failed
     */
    function _handleAtomicSubroutine(
        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine,
        bytes[] memory messages
    ) internal returns (IProcessor.SubroutineResult memory) {
        try this._executeAtomicSubroutine(atomicSubroutine, messages) returns (uint256 totalExecuted) {
            return IProcessor.SubroutineResult({succeeded: true, executedCount: totalExecuted, errorData: ""});
        } catch (bytes memory err) {
            return IProcessor.SubroutineResult({succeeded: false, executedCount: 0, errorData: err});
        }
    }

    /**
     * @notice Executes functions in a non-atomic subroutine until one fails
     * @dev Processes functions one by one, stopping at first failure
     * @param nonAtomicSubroutine The non-atomic subroutine to execute
     * @param messages The messages to be sent for each contract call
     * @return result Contains execution count and error data if any failed
     */
    function _handleNonAtomicSubroutine(
        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine,
        bytes[] memory messages
    ) internal returns (IProcessor.SubroutineResult memory) {
        uint256 executedCount = 0;
        bytes memory errorData;
        bool succeeded = true;

        // Execute each function until one fails
        for (uint256 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
            (bool success, bytes memory err) = nonAtomicSubroutine.functions[i].contractAddress.call(messages[i]);

            if (success) {
                executedCount++;
            } else {
                succeeded = false;
                errorData = err;
                break;
            }
        }

        return IProcessor.SubroutineResult({succeeded: succeeded, executedCount: executedCount, errorData: errorData});
    }

    /**
     * @notice External function that executes the atomic subroutine and reverts if any fail
     * @dev External to allow try-catch pattern for atomicity
     * @param atomicSubroutine The atomic subroutine to execute
     * @param messages The messages to be sent for each contract call
     * @return totalExecuted Number of functions executed
     */
    function _executeAtomicSubroutine(
        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine,
        bytes[] memory messages
    ) external returns (uint256) {
        // Only allow calls from the contract itself, need this extra protection to prevent external access
        // This is necessary because the function is external and can be called by anyone
        // It's external to allow try-catch pattern for atomicity
        if (msg.sender != address(this)) {
            revert ProcessorErrors.UnauthorizedAccessError();
        }

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
            (bool success, bytes memory err) = atomicSubroutine.functions[i].contractAddress.call(messages[i]);
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
