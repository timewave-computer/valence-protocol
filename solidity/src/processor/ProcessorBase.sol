// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {IProcessorMessageTypes} from "./interfaces/IProcessorMessageTypes.sol";
import {IProcessor} from "./interfaces/IProcessor.sol";
import {IMailbox} from "hyperlane/interfaces/IMailbox.sol";
import {ICallback} from "./interfaces/ICallback.sol";
import {ProcessorEvents} from "./libs/ProcessorEvents.sol";

abstract contract ProcessorBase is Ownable {
    /**
     * @notice The authorization contract that can send messages from the main domain
     * @dev Stored as bytes32 to handle cross-chain address representation
     */
    bytes32 public immutable authorizationContract;

    /**
     * @notice The only address allowed to deliver messages to this processor
     * @dev This should be the Hyperlane mailbox contract
     */
    IMailbox public immutable mailbox;

    /**
     * @notice The origin domain ID for sending the callbacks via Hyperlane
     * @dev This is the ID of the domain the authorization contract is deployed on (Neutron ID) - Check: https://hyp-v3-docs-er9k07ozr-abacus-works.vercel.app/docs/reference/domains
     */
    uint32 public immutable originDomain;

    /**
     * @notice The addresses authorized to interact with the processor contract directly
     */
    mapping(address => bool) public authorizedAddresses;

    /**
     * @notice Indicates if the processor is currently paused
     */
    bool public paused;

    /**
     * @notice Initializes the state variables
     * @param _authorizationContract The authorization contract address in bytes32. If we dont want to use Hyperlane, just pass empty bytes here.
     * @param _mailbox The Hyperlane mailbox address. If we don't want to use Hyperlane, we can set it to address(0).
     * @param _originDomain The origin domain ID for sending callbacks. If address(0) is used for mailbox, this will be not be used.
     * @param _authorizedAddresses The addresses authorized to interact with the processor directly
     */
    constructor(
        bytes32 _authorizationContract,
        address _mailbox,
        uint32 _originDomain,
        address[] memory _authorizedAddresses
    ) Ownable(msg.sender) {
        authorizationContract = _authorizationContract;
        mailbox = IMailbox(_mailbox);
        originDomain = _originDomain;

        for (uint8 i = 0; i < _authorizedAddresses.length; i++) {
            authorizedAddresses[_authorizedAddresses[i]] = true;
        }
    }

    /**
     * @notice Handles incoming messages from an authorized addresses
     * @param _body The message payload
     */
    function execute(bytes calldata _body) external payable virtual;

    /**
     * @notice Handles pause messages
     */
    function _handlePause() internal {
        paused = true;
        emit ProcessorEvents.ProcessorWasPaused();
    }

    /**
     * @notice Handles resume messages
     */
    function _handleResume() internal {
        paused = false;
        emit ProcessorEvents.ProcessorWasResumed();
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
            return IProcessor.SubroutineResult({
                succeeded: true,
                expired: false,
                executedCount: totalExecuted,
                errorData: ""
            });
        } catch (bytes memory err) {
            return IProcessor.SubroutineResult({succeeded: false, expired: false, executedCount: 0, errorData: err});
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
        uint32 executedCount = 0;
        bytes memory errorData;
        bool succeeded = true;

        // Execute each function until one fails
        for (uint8 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
            address targetContract = nonAtomicSubroutine.functions[i].contractAddress;
            // Check contract existence, need to do this check because in EVM
            // the .call will still return true even if the contract does not exist
            // It fails on reverts/requires/out of gas
            uint256 size;
            assembly {
                size := extcodesize(targetContract)
            }
            if (size == 0) {
                succeeded = false;
                errorData = "Contract does not exist";
                break;
            }

            (bool success, bytes memory err) = targetContract.call(messages[i]);

            if (success) {
                executedCount++;
            } else {
                succeeded = false;
                errorData = err;
                break;
            }
        }

        return IProcessor.SubroutineResult({
            succeeded: succeeded,
            expired: false,
            executedCount: executedCount,
            errorData: errorData
        });
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
            revert ProcessorErrors.UnauthorizedAccess();
        }

        for (uint8 i = 0; i < atomicSubroutine.functions.length; i++) {
            address targetContract = atomicSubroutine.functions[i].contractAddress;
            // Check contract existence, need to do this check because in EVM
            // the .call function will still return true even if the contract does not exist
            // It fails on reverts/requires/out of gas
            uint256 size;
            assembly {
                size := extcodesize(targetContract)
            }
            if (size == 0) {
                revert("Contract does not exist");
            }

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
            (bool success, bytes memory err) = targetContract.call(messages[i]);
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

    /**
     * @notice Combines callback building and sending into a single atomic operation
     * @dev This function serves as a convenience wrapper that ensures callbacks are properly
     *      built and sent in sequence. It helps maintain the atomicity of the callback process
     *      by handling both operations in a single function call.
     * @param callbackReceiver The address that will receive the callback
     * @param executionId The unique identifier for the execution being reported
     * @param subroutineResult Contains all execution outcomes including success status,
     *        execution count, and any error data from the subroutine execution
     */
    function _buildAndSendCallback(
        address callbackReceiver,
        uint64 executionId,
        IProcessor.SubroutineResult memory subroutineResult
    ) internal {
        IProcessor.Callback memory callback = _buildCallback(executionId, subroutineResult);
        _sendCallback(callbackReceiver, callback);
    }

    /**
     * @notice Builds a callback structure containing execution results and encodes it
     * @dev This function processes the results of a subroutine execution and packages it into a standardized format
     * @param executionId Unique identifier for this execution instance
     * @param subroutineResult Contains the execution results including success status, executed count, and any error data
     * @return callback The callback structure containing the execution outcome
     */
    function _buildCallback(uint64 executionId, IProcessor.SubroutineResult memory subroutineResult)
        internal
        pure
        returns (IProcessor.Callback memory)
    {
        // Determine the execution result based on the following rules:
        // - If succeeded = true -> Success (all operations completed)
        // - If succeeded = false and expired = true -> Expired (some operations might have been completed)
        // - If succeeded = false, expired = false and executedCount = 0 -> Rejected (nothing executed)
        // - If succeeded = false, expired = false and executedCount > 0 -> PartiallyExecuted (some operations completed)
        IProcessor.ExecutionResult executionResult;
        if (subroutineResult.succeeded) {
            executionResult = IProcessor.ExecutionResult.Success;
        } else if (subroutineResult.expired) {
            executionResult = IProcessor.ExecutionResult.Expired;
        } else if (subroutineResult.executedCount == 0) {
            executionResult = IProcessor.ExecutionResult.Rejected;
        } else {
            executionResult = IProcessor.ExecutionResult.PartiallyExecuted;
        }

        // Construct the callback structure and return it
        return IProcessor.Callback({
            executionId: executionId,
            executionResult: executionResult,
            executedCount: subroutineResult.executedCount,
            data: subroutineResult.errorData
        });
    }

    /**
     * @notice Sends an encoded callback to the designated mailbox contract
     * @dev This function handles the actual dispatch of callback data through the
     *      cross-domain messaging system. It uses the mailbox contract to send
     *      the message back to the origin domain and emits an event for tracking.
     * @param callbackReceiver The address that will receive the callback
     * @param callback The callback structure to be sent
     */
    function _sendCallback(address callbackReceiver, IProcessor.Callback memory callback) internal {
        // Encode the entire callback structure into bytes for transmission
        // Using abi.encode ensures proper encoding of all struct members
        // This encoded data can be decoded later by the decoder
        bytes memory encodedCallback = abi.encode(callback);

        // If the sender was the mailbox, we send it back to the mailbox, otherwise we send it to the contract that initiated the execution
        if (callbackReceiver == address(mailbox)) {
            mailbox.dispatch(originDomain, authorizationContract, encodedCallback);
        } else {
            // Send the callback to the designated receiver, but we don't revert on failure
            (bool success,) =
                callbackReceiver.call(abi.encodeWithSelector(ICallback.handleCallback.selector, encodedCallback));
            success; // No-op; the variable is not being used for anything
        }
        // Emit an event to track the callback transmission
        emit ProcessorEvents.CallbackSent(callback.executionId, callback.executionResult, callback.executedCount);
    }

    /**
     * @notice Adds an address to the list of authorized addresses
     * @dev Only callable by the contract owner
     * @param _address The address to be authorized
     */
    function addAuthorizedAddress(address _address) public onlyOwner {
        // Check that address is not the zero address
        if (_address == address(0)) {
            revert ProcessorErrors.InvalidAddress();
        }

        // Check that address is not already authorized
        if (authorizedAddresses[_address]) {
            revert ProcessorErrors.AddressAlreadyAuthorized();
        }

        authorizedAddresses[_address] = true;
        emit ProcessorEvents.AuthorizedAddressAdded(_address);
    }

    /**
     * @notice Removes an address from the list of authorized addresses
     * @dev Only callable by the contract owner
     * @param _address The address to be removed from the authorized list
     */
    function removeAuthorizedAddress(address _address) public onlyOwner {
        // Check that address is currently authorized
        if (!authorizedAddresses[_address]) {
            revert ProcessorErrors.AddressNotAuthorized();
        }

        delete authorizedAddresses[_address];
        emit ProcessorEvents.AuthorizedAddressRemoved(_address);
    }
}
