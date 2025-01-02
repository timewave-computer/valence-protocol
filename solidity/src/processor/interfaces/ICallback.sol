// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/// @title Callback Interface
/// @notice Interface for handling processor callbacks
/// @dev Must be implemented by contracts if they want to handle processor callbacks
interface ICallback {
    /// @notice Handles incoming callback data from the processor
    /// @param callbackData ABI encoded callback parameters
    /// @dev Validate and process callback data appropriately
    function handleCallback(bytes memory callbackData) external;
}
