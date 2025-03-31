// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import "../../src/libraries/interfaces/cctp/ITokenMessenger.sol";

// Add the interface definition for IERC20
interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

/// @title MockTokenMessenger
/// @notice Mock implementation of the ITokenMessenger interface with stubs for testing.
contract MockTokenMessenger is ITokenMessenger {
    function depositForBurn(uint256 amount, uint32 destinationDomain, bytes32 mintRecipient, address burnToken)
        external
        returns (uint64)
    {
        require(amount > 0, "Amount must be gt 0");
        require(mintRecipient != bytes32(0), "Mint recipient must != 0");

        // Transfer tokens to this contract
        IERC20(burnToken).transferFrom(msg.sender, address(this), amount);

        // emit event
        emit DepositForBurn(
            0, burnToken, amount, msg.sender, mintRecipient, destinationDomain, bytes32(uint256(0x1)), bytes32(0)
        );

        return 0;
    }

    function depositForBurnWithCaller(
        uint256 amount,
        uint32 destinationDomain,
        bytes32 mintRecipient,
        address burnToken,
        bytes32 destinationCaller
    ) external returns (uint64) {
        require(amount > 0, "Amount must be gt 0");
        require(mintRecipient != bytes32(0), "Mint recipient must != 0");

        // Transfer tokens to this contract
        IERC20(burnToken).transferFrom(msg.sender, address(this), amount);

        // emit event
        emit DepositForBurn(
            0, burnToken, amount, msg.sender, mintRecipient, destinationDomain, bytes32(uint256(0x1)), destinationCaller
        );

        return 0;
    }

    function replaceDepositForBurn(
        bytes calldata originalMessage,
        bytes calldata originalAttestation,
        bytes32 newDestinationCaller,
        bytes32 newMintRecipient
    ) external {}

    function handleReceiveMessage(uint32, bytes32, bytes calldata) external pure returns (bool) {
        return true;
    }
}
