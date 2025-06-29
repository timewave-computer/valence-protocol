// SPDX-License-Identifier: Apache-2.0
// Taken from: https://github.com/unionlabs/union/
pragma solidity ^0.8.28;

/**
 * @dev Interface with everything we need to interact with Union apps.
 */
interface IUnion {
    struct Instruction {
        uint8 version;
        uint8 opcode;
        bytes operand;
    }

    struct FungibleAssetOrder {
        bytes sender; // Source chain sender address
        bytes receiver; // Destination chain receiver address
        bytes baseToken; // Token being sent
        uint256 baseAmount; // Amount being sent
        string baseTokenSymbol; // Token symbol for wrapped asset
        string baseTokenName; // Token name for wrapped asset
        uint8 baseTokenDecimals; // Token decimals for wrapped asset
        uint256 baseTokenPath; // Origin path for unwrapping
        bytes quoteToken; // Token requested in return
        uint256 quoteAmount; // Minimum amount requested
    }

    function send(
        uint32 channelId,
        uint64 timeoutHeight,
        uint64 timeoutTimestamp,
        bytes32 salt,
        Instruction calldata instruction
    ) external;
}
