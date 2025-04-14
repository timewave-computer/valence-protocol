// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/**
 * @title LibraryProxy
 * @dev Minimal proxy contract that delegates calls to a Forwarder implementation
 * Can be deployed via CREATE2 for deterministic addressing
 */
contract LibraryProxy {
    // Admin address can initialize the proxy
    address public admin;
    
    // Implementation address that calls will be delegated to
    address public implementation;
    
    // Flag to prevent re-initialization
    bool public initialized;
    
    /**
     * @dev Constructor sets the deployer as admin
     * No complex initialization that could revert
     */
    constructor(address _admin) {
        admin = _admin;
    }
    
    /**
     * @dev Initialize the proxy with the implementation address
     * @param _implementation Address of the Forwarder implementation
     */
    function initialize(address _implementation) external {
        require(msg.sender == admin, "Only admin can initialize");
        require(!initialized, "Already initialized");
        require(_implementation != address(0), "Implementation cannot be zero address");
        
        initialized = true;
        implementation = _implementation;
    }
    
    /**
     * @dev Fallback function delegates all calls to the implementation
     */
    fallback() external payable {
        _delegate(implementation);
    }
    
    /**
     * @dev Receive function to handle ETH transfers
     */
    receive() external payable {
        _delegate(implementation);
    }
    
    /**
     * @dev Internal function that delegates execution to implementation contract
     * @param _implementation Address to delegate calls to
     */
    function _delegate(address _implementation) internal {
        require(_implementation != address(0), "Implementation not set");
        
        // solhint-disable-next-line no-inline-assembly
        assembly {
            // Copy msg.data. We take full control of memory in this inline assembly
            // block because it will not return to Solidity code. We overwrite the
            // Solidity scratch pad at memory position 0.
            calldatacopy(0, 0, calldatasize())
            
            // Call the implementation.
            // out and outsize are 0 because we don't know the size yet.
            let result := delegatecall(gas(), _implementation, 0, calldatasize(), 0, 0)
            
            // Copy the returned data.
            returndatacopy(0, 0, returndatasize())
            
            switch result
            // delegatecall returns 0 on error.
            case 0 { revert(0, returndatasize()) }
            default { return(0, returndatasize()) }
        }
    }
}