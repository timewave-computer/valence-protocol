// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";
import {QueueMap} from "./libs/QueueMap.sol";
import {ProcessorBase} from "./ProcessorBase.sol";
import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {ProcessorEvents} from "./libs/ProcessorEvents.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title Processor
 * @notice (unimplemented) A full implementation of a Processor.
 * @dev Implements IMessageRecipient for Hyperlane message handling, ProcessorBase for core shared processor logic and ReentrancyGuard to prevent re-entrancy attacks.
 */
contract Processor is IMessageRecipient, ProcessorBase, ReentrancyGuard {
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
    function execute(bytes calldata _body) external payable override nonReentrant {
        // TODO: Implement the execute function
    }
}
