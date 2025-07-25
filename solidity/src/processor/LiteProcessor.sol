// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";
import {ProcessorMessageDecoder} from "./libs/ProcessorMessageDecoder.sol";
import {IProcessorMessageTypes} from "./interfaces/IProcessorMessageTypes.sol";
import {IProcessor} from "./interfaces/IProcessor.sol";
import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {ProcessorBase} from "./ProcessorBase.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title LiteProcessor
 * @notice A lightweight processor for handling cross-chain messages with atomic and non-atomic execution
 * @dev Implements IMessageRecipient for Hyperlane message handling, ProcessorBase for core shared processor logic and ReentrancyGuard to prevent re-entrancy attacks.
 */
contract LiteProcessor is IMessageRecipient, ProcessorBase, ReentrancyGuard {
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
    ) ProcessorBase(_authorizationContract, _mailbox, _originDomain, _authorizedAddresses) {}

    // ============ External Functions ============

    /**
     * @notice Handles incoming messages from the Hyperlane mailbox
     * @param _origin The origin domain ID
     * @param _sender The sender's address in bytes32
     * @param _body The message payload
     */
    function handle(uint32 _origin, bytes32 _sender, bytes calldata _body) external payable override {
        // Verify sender is authorized mailbox
        if (msg.sender != address(mailbox)) {
            revert ProcessorErrors.UnauthorizedAccess();
        }

        // Verify origin is the expected domain
        if (_origin != originDomain) {
            revert ProcessorErrors.InvalidOriginDomain();
        }

        // Verify message is from authorization contract
        if (_sender != authorizationContract) {
            revert ProcessorErrors.NotAuthorizationContract();
        }

        // Decode and route message to appropriate handler
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage = ProcessorMessageDecoder.decode(_body);
        _handleMessageType(decodedMessage);
    }

    /**
     * @notice Handles incoming messages from an authorized addresses
     * @param _body The message payload
     */
    function execute(bytes calldata _body) external override nonReentrant {
        // Verify sender is authorized address
        require(authorizedAddresses[msg.sender], ProcessorErrors.UnauthorizedAccess());

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
        } else {
            // InsertMsgs and EvictMsgs are not supported in LiteProcessor because there are no queues
            revert ProcessorErrors.UnsupportedOperation();
        }
    }

    /**
     * @notice Processes SendMsgs operations based on subroutine type
     * @dev Decodes and routes to appropriate subroutine handler
     * @param decodedMessage The decoded processor message
     */
    function _handleSendMsgs(IProcessorMessageTypes.ProcessorMessage memory decodedMessage) internal {
        // Check if the processor is paused
        if (paused) {
            revert ProcessorErrors.ProcessorPaused();
        }

        IProcessorMessageTypes.SendMsgs memory sendMsgs =
            abi.decode(decodedMessage.message, (IProcessorMessageTypes.SendMsgs));

        IProcessor.SubroutineResult memory result;
        // Check if there is an expiration time and if there is, if it already passed
        if (sendMsgs.expirationTime != 0 && sendMsgs.expirationTime < block.timestamp) {
            result =
                IProcessor.SubroutineResult({succeeded: false, expired: true, executedCount: 0, errorData: bytes("")});
            _buildAndSendCallback(msg.sender, sendMsgs.executionId, result);
            return;
        }

        if (sendMsgs.subroutine.subroutineType == IProcessorMessageTypes.SubroutineType.Atomic) {
            IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
                abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));
            result = _handleAtomicSubroutine(atomicSubroutine, sendMsgs.messages);
        } else {
            IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
                abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

            result = _handleNonAtomicSubroutine(nonAtomicSubroutine, sendMsgs.messages);
        }

        // Send callback only if the address that executed this function is a Smart Contract (e.g Hyperlane mailbox or sub-authorization contract)
        if (msg.sender.code.length > 0) {
            _buildAndSendCallback(msg.sender, sendMsgs.executionId, result);
        }
    }
}
