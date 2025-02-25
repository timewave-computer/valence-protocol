// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import "../../src/libraries/interfaces/cctp/ITokenMessenger.sol";

/// @title MockTokenMessenger
/// @notice Mock implementation of the ITokenMessenger interface with stubs for testing.
contract MockTokenMessenger is ITokenMessenger {
    function depositForBurn(uint256, uint32, bytes32, address) external pure returns (uint64 _nonce) {
        return 0;
    }

    function depositForBurnWithCaller(uint256, uint32, bytes32, address, bytes32)
        external
        pure
        returns (uint64 nonce)
    {
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
