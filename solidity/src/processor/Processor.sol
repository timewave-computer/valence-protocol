// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";
import {QueueMap} from "./libs/QueueMap.sol";
import {ProcessorBase} from "./ProcessorBase.sol";
import {ProcessorErrors} from "./libs/ProcessorErrors.sol";
import {ProcessorEvents} from "./libs/ProcessorEvents.sol";

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
     *      and passing the necessary parameters for the authorized contract and mailbox.
     * @param _authorizationContract The address of the authorized contract, represented as a bytes32 value.
     * @param _mailbox The address of the Hyperlane mailbox contract.
     * @param _originDomain The origin domain ID for sending the callbacks via Hyperlane.
     */
    constructor(bytes32 _authorizationContract, address _mailbox, uint32 _originDomain)
        ProcessorBase(_authorizationContract, _mailbox, _originDomain)
    {}

    // Implement the handle function
    function handle(uint32 _origin, bytes32 _sender, bytes calldata _body) external payable override {
        // Only mailbox can call this function
        if (msg.sender != address(mailbox)) {
            revert ProcessorErrors.UnauthorizedAccess();
        }

        // Check that the sender of the message is the authorization contract
        if (_sender != authorizationContract) {
            revert ProcessorErrors.NotAuthorizationContract();
        }

        emit ProcessorEvents.MessageReceived(_origin, _sender, _body);
    }
}
