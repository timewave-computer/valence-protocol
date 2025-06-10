// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/// JIT (Just-In-Time) Account implementation for factory-created accounts
/// 
/// Provides controller-bound execution where only the designated controller 
/// or approved libraries can perform operations.
contract JitAccount {
    // Account configuration
    address public immutable controller;
    
    // Library approval mapping
    mapping(address => bool) public approvedLibraries;
    
    // Events
    event LibraryApproved(address indexed lib);
    event LibraryRemoved(address indexed lib);
    event MessagesExecuted(address indexed sender, uint256 count);
    
    // Errors
    error Unauthorized();
    error CallFailed(bytes returnData);
    
    modifier onlyController() {
        if (msg.sender != controller) revert Unauthorized();
        _;
    }
    
    modifier onlyAuthorized() {
        if (msg.sender != controller && !approvedLibraries[msg.sender]) {
            revert Unauthorized();
        }
        _;
    }
    
    constructor(address _controller) {
        controller = _controller;
    }
    
    /// Approve a library to execute messages on behalf of this account
    /// Only the controller can approve libraries
    function approveLibrary(address lib) external onlyController {
        approvedLibraries[lib] = true;
        emit LibraryApproved(lib);
    }
    
    /// Remove approval for a library
    /// Only the controller can remove library approvals
    function removeLibrary(address lib) external onlyController {
        approvedLibraries[lib] = false;
        emit LibraryRemoved(lib);
    }
    
    /// Execute arbitrary calls through this account
    /// Can be called by controller or approved libraries
    function execute(
        address[] calldata targets,
        bytes[] calldata data,
        uint256[] calldata values
    ) external payable onlyAuthorized {
        if (targets.length != data.length || targets.length != values.length) {
            revert("Array length mismatch");
        }
        
        for (uint256 i = 0; i < targets.length; i++) {
            (bool success, bytes memory returnData) = targets[i].call{value: values[i]}(data[i]);
            if (!success) {
                revert CallFailed(returnData);
            }
        }
        
        emit MessagesExecuted(msg.sender, targets.length);
    }
    
    /// Get the controller address
    function getController() external view returns (address) {
        return controller;
    }
    
    /// Check if a library is approved
    function isLibraryApproved(address lib) external view returns (bool) {
        return approvedLibraries[lib];
    }
    
    /// Receive ETH
    receive() external payable {}
}
