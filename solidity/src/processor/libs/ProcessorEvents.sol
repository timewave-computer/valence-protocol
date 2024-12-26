// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IProcessor} from "../interfaces/IProcessor.sol";

library ProcessorEvents {
    /**
     * @notice Emitted when a message is received by the processor
     * @param origin The domain ID where the message originated
     * @param sender The sender's address in bytes32 format
     * @param body The raw message bytes
     */
    event MessageReceived(uint32 indexed origin, bytes32 indexed sender, bytes body);

    /**
     * @notice Emitted when the processor is paused
     */
    event ProcessorPaused();

    /**
     * @notice Emitted when the processor is resumed
     */
    event ProcessorResumed();

    /**
     * @notice Emitted when a callback is built for the hyperlane mailbox
     * @param executionId The Execution ID of the message(s) that triggered the callback
     * @param result The outcome of the execution
     */
    event CallbackBuilt(uint256 indexed executionId, IProcessor.ExecutionResult result);
}
