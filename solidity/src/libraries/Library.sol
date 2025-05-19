// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title Library
 * @dev Abstract contract that defines the base library functionality.
 * Any library will inherit from this.
 * Inherits from OpenZeppelin's Ownable for basic access control
 */
abstract contract Library is Ownable {
    // Address of the processor that will execute functions on this library
    address public processor;

    /**
     * @dev Restricts function to be called only by the processor address
     */
    modifier onlyProcessor() {
        require(msg.sender == processor, "Only the processor can call this function");
        _;
    }

    /**
     * @dev Constructor initializes the library with owner, processor, and initial configuration
     * @param _owner The initial owner of the contract
     * @param _processor The initial processor address
     * @param _config Initial configuration data for the library
     * @notice Calls updateConfig to set initial library configuration
     */
    constructor(address _owner, address _processor, bytes memory _config) Ownable(_owner) {
        // Set the processor address
        processor = _processor;
        // Initialize configuration using internal initialization (no access control)
        _initConfig(_config);
    }

    /**
     * @dev Allows the owner to update the processor address
     * @param _processor New processor address
     */
    function updateProcessor(address _processor) external onlyOwner {
        processor = _processor;
    }

    /**
     * @dev Internal function for initialization during construction
     * @param _config Configuration data to be applied
     */
    function _initConfig(bytes memory _config) internal virtual;

    /**
     * @dev Updates the library configuration
     * @param _config Configuration data to be applied
     * @notice Must be implemented by each library
     * @dev Should validate function before applying configuration
     */
    function updateConfig(bytes memory _config) public virtual;

    /// @notice Fallback function that reverts all calls to non-existent functions
    /// @dev Called when no other function matches the function signature
    /// @dev Add this to your contract to explicitly fail calls to wrong/non-existent
    fallback() external {
        revert("Function not found");
    }
}
