// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";
import {QueueMap} from "./libs/QueueMap.sol";
import {ProcessorBase} from "./ProcessorBase.sol";
import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {ProcessorEvents} from "./libs/ProcessorEvents.sol";
import {IProcessorMessageTypes} from "./interfaces/IProcessorMessageTypes.sol";
import {ProcessorMessageDecoder} from "./libs/ProcessorMessageDecoder.sol";

contract Processor is IMessageRecipient, ProcessorBase {
    // Use the library for the Queue type
    using QueueMap for QueueMap.Queue;

    // Declare the two queues
    QueueMap.Queue private highPriorityQueue;
    QueueMap.Queue private mediumPriorityQueue;

    // ============ Constructor ============
    /**
     * @notice Initializes the LiteProcessor contract
     * @dev The constructor initializes the LiteProcessor by calling the base contract constructor
     *      and passing the necessary parameters for the authorization contract and mailbox.
     * @param _authorizationContract The address of the authorization contract, represented as a bytes32 value.
     * @param _mailbox The address of the Hyperlane mailbox contract.
     * @param _originDomain The origin domain ID for sending the callbacks via Hyperlane.
     * @param _authorizedAddresses The list of authorized addresses that can call the processor directly.
     */
    constructor(
        bytes32 _authorizationContract,
        address _mailbox,
        uint32 _originDomain,
        address[] memory _authorizedAddresses
    ) ProcessorBase(_authorizationContract, _mailbox, _originDomain, _authorizedAddresses) {
        // Initialize both queues with unique namespaces
        mediumPriorityQueue = QueueMap.createQueue("MED");
        highPriorityQueue = QueueMap.createQueue("HIGH");
    }

    // Implement the handle function
    function handle(uint32 _origin, bytes32 _sender, bytes calldata /*_body*/ ) external payable override {
        // Only mailbox can call this function
        if (msg.sender != address(mailbox)) {
            revert ProcessorErrors.UnauthorizedAccess();
        }

        // Verify origin is the expected domain
        if (_origin != originDomain) {
            revert ProcessorErrors.InvalidOriginDomain();
        }

        // Check that the sender of the message is the authorization contract
        if (_sender != authorizationContract) {
            revert ProcessorErrors.NotAuthorizationContract();
        }
    }

    /**
     * @notice Handles incoming messages from an authorized addresses
     * @param _body The message payload
     */
    function execute(bytes calldata _body) external payable override {
        // Check if the processor is paused
        if (paused) {
            revert ProcessorErrors.ProcessorPaused();
        }

        // Check if the sender is authorized
        if (!authorizedAddresses[msg.sender]) {
            revert ProcessorErrors.UnauthorizedAccess();
        }

        // Decode the processor message
        IProcessorMessageTypes.ProcessorMessage memory message = ProcessorMessageDecoder.decode(_body);

        // Handle the message based on its type
        if (message.messageType == IProcessorMessageTypes.ProcessorMessageType.Pause) {
            _handlePause();
        } else if (message.messageType == IProcessorMessageTypes.ProcessorMessageType.Resume) {
            _handleResume();
        } else if (message.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            _handleSendMsgs(ProcessorMessageDecoder.decodeSendMsgs(message.message));
        } else if (message.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs) {
            _handleInsertMsgs(ProcessorMessageDecoder.decodeInsertMsgs(message.message));
        } else if (message.messageType == IProcessorMessageTypes.ProcessorMessageType.EvictMsgs) {
            _handleEvictMsgs(ProcessorMessageDecoder.decodeEvictMsgs(message.message));
        } else {
            revert ProcessorErrors.UnsupportedOperation();
        }
    }

    /**
     * @notice Handles SendMsgs by adding them to the appropriate queue
     * @param sendMsgs The SendMsgs message to process
     */
    function _handleSendMsgs(IProcessorMessageTypes.SendMsgs memory sendMsgs) internal {
        // Encode the complete message for storage
        bytes memory encodedMessage = abi.encode(sendMsgs);
        
        if (sendMsgs.priority == IProcessorMessageTypes.Priority.High) {
            highPriorityQueue.pushBack(encodedMessage);
        } else {
            mediumPriorityQueue.pushBack(encodedMessage);
        }
        
        emit ProcessorEvents.MessageBatchAdded(sendMsgs.executionId, sendMsgs.priority);
    }

    /**
     * @notice Handles InsertMsgs by inserting them at a specific position in the queue
     * @param insertMsgs The InsertMsgs message to process
     */
    function _handleInsertMsgs(IProcessorMessageTypes.InsertMsgs memory insertMsgs) internal {
        // Encode the complete message for storage
        bytes memory encodedMessage = abi.encode(insertMsgs);
        
        if (insertMsgs.priority == IProcessorMessageTypes.Priority.High) {
            highPriorityQueue.insertAt(insertMsgs.queuePosition, encodedMessage);
        } else {
            mediumPriorityQueue.insertAt(insertMsgs.queuePosition, encodedMessage);
        }
        
        emit ProcessorEvents.MessageBatchAdded(insertMsgs.executionId, insertMsgs.priority);
    }

    /**
     * @notice Handles EvictMsgs by removing messages from a specific queue position
     * @param evictMsgs The EvictMsgs message to process
     */
    function _handleEvictMsgs(IProcessorMessageTypes.EvictMsgs memory evictMsgs) internal {
        if (evictMsgs.priority == IProcessorMessageTypes.Priority.High) {
            highPriorityQueue.removeAt(evictMsgs.queuePosition);
        } else {
            mediumPriorityQueue.removeAt(evictMsgs.queuePosition);
        }
        
        emit ProcessorEvents.MessageBatchRemoved(evictMsgs.priority, evictMsgs.queuePosition);
    }
}
