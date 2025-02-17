// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

interface IProcessor {
    /**
     * @notice Result of a subroutine execution
     * @param succeeded Whether all functions executed successfully
     * @param expired Whether the execution failed because it expired
     * @param executedCount Number of successfully executed functions before failure or completion. For atomic subroutines, this will be the total count if all succeeded
     * @param errorData The error data from the last failed function, empty if all succeeded
     */
    struct SubroutineResult {
        bool succeeded;
        bool expired;
        uint256 executedCount;
        bytes errorData;
    }

    /**
     * @notice Represents the callback after a subroutine execution
     * @param executionId The Execution ID of the message(s) that triggered the callback
     * @param executionResult The outcome of the execution (Success, Rejected, or PartiallyExecuted)
     * @param data Additional data related to the callback execution, if any
     */
    struct Callback {
        uint64 executionId;
        ExecutionResult executionResult;
        uint256 executedCount;
        bytes data;
    }

    /**
     * @notice Enum representing the possible results of a subroutine execution
     * @dev Used in Callback struct to indicate the overall status of the execution
     * @param Success Indicates that all functions were executed
     * @param Rejected Indicates that nothing was executed
     * @param PartiallyExecuted Indicates that the execution was partially successful (some functions executed, only for non-atomic subroutines)
     * @param Expired Indicates that the execution was not before the expiration time
     */
    enum ExecutionResult {
        Success,
        Rejected,
        PartiallyExecuted,
        Expired
    }

    /**
     * @notice Represents the details of a rejected execution result
     * @dev This struct is used to store the error data in case of rejection during subroutine execution
     * @param errorData Contains the raw error data from the failed execution
     */
    struct RejectedResult {
        bytes errorData;
    }

    /**
     * @notice Represents the details of a partially executed result (only for non-atomic subroutines)
     * @dev This struct stores information about the partial success of the execution
     * @param executedCount The number of functions that were executed successfully before failure
     * @param errorData Contains the error data from the first failed function
     */
    struct PartiallyExecutedResult {
        uint256 executedCount;
        bytes errorData;
    }

    /**
     * @notice Represents the details of an expired execution result
     * @dev This struct stores information about the expiration of the execution
     * @param executedCount The number of functions that were executed before expiration
     */
    struct ExpiredResult {
        uint256 executedCount;
    }
}
