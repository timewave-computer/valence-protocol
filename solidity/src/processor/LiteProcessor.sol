// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IMessageRecipient} from "hyperlane/interfaces/IMessageRecipient.sol";

contract LiteProcessor is IMessageRecipient {
    // Authorization contract will be the sender of the messages on the main domain
    // It's the hex representation of the address in the main domain
    bytes32 public authorizationContract;

    // Add mailbox address which will be the only address that can send messages to the processor
    address public mailbox;

    // Flag to check if the processor is paused
    bool public paused;

    event MessageReceived(uint32 indexed origin, bytes32 indexed sender, bytes body);

    // Custom errors
    error UnauthorizedAccess();
    error NotAuthorizationContract();
    error InvalidAddress();
    error ProcessorPaused();
    error UnsupportedOperation();

    constructor(bytes32 _authorizationContract, address _mailbox) {
        // Check for zero addresses
        if (_mailbox == address(0)) {
            revert InvalidAddress();
        }

        // Set authorization contract and mailbox
        authorizationContract = _authorizationContract;
        mailbox = _mailbox;
    }

    // Implement the handle function
    function handle(uint32 _origin, bytes32 _sender, bytes calldata _body) external payable override {
        // Only mailbox can call this function
        if (msg.sender != mailbox) {
            revert UnauthorizedAccess();
        }

        // Check that the sender of the message is the authorization contract
        if (_sender != authorizationContract) {
            revert NotAuthorizationContract();
        }

        emit MessageReceived(_origin, _sender, _body);
    }
}
