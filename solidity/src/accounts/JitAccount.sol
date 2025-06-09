// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/// Minimal JIT Account implementation for testing
contract JitAccount {
    address public controller;
    uint8 public accountType;
    
    constructor(address _controller, uint8 _accountType) {
        controller = _controller;
        accountType = _accountType;
    }
    
    function getController() external view returns (address) {
        return controller;
    }
    
    function getAccountType() external view returns (uint8) {
        return accountType;
    }
} 