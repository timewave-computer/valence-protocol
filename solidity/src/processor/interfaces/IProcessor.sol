// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

interface IProcessor {
    /**
     * @notice Result of a subroutine execution
     * @param succeeded Whether all functions executed successfully
     * @param executedCount Number of successfully executed functions before failure or completion. For atomic subroutines, this will be the total count if all succeeded
     * @param errorData The error data from the last failed function, empty if all succeeded
     */
    struct SubroutineResult {
        bool succeeded;
        uint256 executedCount;
        bytes errorData;
    }
}
