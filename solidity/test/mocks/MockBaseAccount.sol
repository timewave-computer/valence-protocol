// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

contract MockBaseAccount {
    struct ExecuteParams {
        address target; // The address of the contract to execute
        uint256 value; // The amount of Ether to send with the call
        bytes data; // The calldata for the function to be executed
    }

    ExecuteParams[] public executeParams;

    function execute(address target, uint256 value, bytes calldata data) external returns (bytes memory result) {
        ExecuteParams memory params = ExecuteParams({target: target, value: value, data: data});

        executeParams.push(params);

        return new bytes(0);
    }

    /// @dev Allows the contract to receive native tokens (e.g. ETH) that can later be used by approved libraries or the owner in execute() calls
    receive() external payable {}
}
