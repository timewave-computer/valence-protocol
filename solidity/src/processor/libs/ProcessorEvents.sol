// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IProcessor} from "../interfaces/IProcessor.sol";

library ProcessorEvents {
    /**
     * @notice Emitted when the processor is paused
     */
    event ProcessorPaused();

    /**
     * @notice Emitted when the processor is resumed
     */
    event ProcessorResumed();

    /**
     * @notice Emitted when a callback is sent to the hyperlane mailbox
     * @param executionId The Execution ID of the message(s) that triggered the callback
     * @param result The outcome of the execution
     * @param executedCount The number of functions that were executed successfully before failure or completion
     */
    event CallbackSent(uint64 indexed executionId, IProcessor.ExecutionResult result, uint256 executedCount);
}
