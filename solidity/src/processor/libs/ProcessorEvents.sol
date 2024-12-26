// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

library ProcessorEvents {
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

    /**
     * @notice Emitted when a callback is sent to the hyperlane mailbox
     */
    event CallbackSent();
}
