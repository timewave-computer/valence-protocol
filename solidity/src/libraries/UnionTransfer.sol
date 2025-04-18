// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IUnion} from "./interfaces/union/IUnion.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {console} from "forge-std/src/console.sol";

/**
 * @title UnionTransfer
 * @dev Contract for transferring tokens using the Union protocol.
 * It leverages an external UCS03-ZKGM protocol to do cross-chain token transfers. It allows arbitrary filling of orders by any party.
 */
contract UnionTransfer is Library {
    /**
     * @dev Configuration struct for token transfer parameters.
     * @param amount The number of tokens to transfer. If set to 0, the entire balance is transferred.
     * @param inputAccount The account from which tokens will be debited.
     * @param recipient The recipient (in Bytes format) on the destination chain where tokens will be received.
     *        For bech32 addresses, it just converts the entire address to bytes. For example the bytes representation
     *        of `bbn14mlpd48k5vkeset4x7f78myz3m47jcaxz9gpnl` would be
     *        `0x62626e31346d6c706434386b35766b657365743478376637386d797a336d34376a6361787a3967706e6c`
     * @param protocolVersion The protocol version to be used. Required for backward compatibility. Allows dispatching between different versions.
     * @param zkGM The zkGM contract.
     * @param transferToken The ERC20 token address that will be transferred.
     * @param transferTokenName The name of the token being transferred (e.g., "Babylon")
     * @param transferTokenSymbol The symbol of the token being transferred. (e.g., "BABY")
     * @param transferTokenDecimals The number of decimals for the token being transferred. (e.g., 6)
     * @param transferTokenUnwrappingPath Origin path for unwrapping, (e.g., 0 for WETH, 1 for BABY...). Related to the origin chain of these tokens.
     * @param quoteToken The token requested in return on destination chain. Bytes conversion of the token.
     *       For example, the quote Token for WETH on Babylon would be `0x62626e31333030736530767775653737686e36733877706836346579366435357a616634386a72766567397761667371756e636e33653473637373677664`
     *       which bytes conversion of "bbn1300se0vwue77hn6s8wph64ey6d55zaf48jrveg9wafsquncn3e4scssgvd" because WETH is a CW20 token on Babylon.
     *       For BABY, on the other side, it would be `0x7562626e` which is the bytes conversion of "ubbn".
     * @param quoteTokenAmount The amount of the quote token requested in return on the destination chain. If set to 0, the same amount as the transferred token is requested.
     * @param channelId The channel ID for the transfer. This is used to identify the specific transfer channel.
     * @param timeout The timeout in seconds for the transfer. For reference, 3 days is being used on btc.union.build (259200 seconds).
     */
    struct UnionTransferConfig {
        uint256 amount;
        BaseAccount inputAccount;
        bytes recipient;
        uint8 protocolVersion;
        IUnion zkGM;
        bytes transferToken;
        string transferTokenName;
        string transferTokenSymbol;
        uint8 transferTokenDecimals;
        uint256 transferTokenUnwrappingPath;
        bytes quoteToken;
        uint256 quoteTokenAmount;
        uint32 channelId;
        uint64 timeout;
    }

    // Holds the current configuration for token transfers
    UnionTransferConfig public config;

    // Counter used for creating unique salt values for each transfer
    uint256 public counter;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the Union transfer.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration by decoding the provided bytes and ensuring no critical addresses are zero.
     * This prevents misconfiguration.
     * @param _config Raw configuration bytes.
     * @return Decoded and validated UnionTransferConfig struct.
     */
    function validateConfig(bytes memory _config) internal pure returns (UnionTransferConfig memory) {
        // Decode the configuration bytes into the UnionTransferConfig struct.
        UnionTransferConfig memory decodedConfig = abi.decode(_config, (UnionTransferConfig));

        // Ensure the zkGM is a valid (non-zero) address.
        if (decodedConfig.zkGM == IUnion(address(0))) {
            revert("zkGM can't be zero address");
        }

        // Ensure the transfer token address is valid.
        if (decodedConfig.transferToken.length == 0) {
            revert("Transfer token can't be empty bytes");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the recipient address is valid.
        if (decodedConfig.recipient.length == 0) {
            revert("Recipient can't be empty bytes");
        }

        // Ensure the transfer token name is not empty.
        if (bytes(decodedConfig.transferTokenName).length == 0) {
            revert("Transfer token name can't be empty");
        }

        // Ensure the transfer token symbol is not empty.
        if (bytes(decodedConfig.transferTokenSymbol).length == 0) {
            revert("Transfer token symbol can't be empty");
        }

        // Ensure the quote token address is valid.
        if (decodedConfig.quoteToken.length == 0) {
            revert("Quote token can't be empty bytes");
        }

        // Ensure timeout is valid (greater than 0).
        if (decodedConfig.timeout == 0) {
            revert("Timeout can't be zero");
        }

        return decodedConfig;
    }

    /**
     * @dev Updates the UnionTransfer configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @dev Executes the token transfer using the UCS03-ZKGM protocol.
     *
     * Steps:
     * 1. Retrieve the current configuration.
     * 2. Convert the transfer token to an address.
     * 3. Check the token balance of the input account to ensure sufficient funds.
     * 4. Determine the quote amount and transfer amount to use.
     * 5. Encode the FungibleAssetOrder for the zkGM.
     * 6. Create the Instruction with the appropriate opcode.
     * 7. Approve the zkGM to spend tokens from the input account.
     * 8. Generate a unique salt for the transaction.
     * 9. Execute the send call to complete the transfer via zkGM.
     *
     * Requirements:
     * - The caller must be the designated processor.
     * - The input account must hold enough tokens for the transfer.
     * - If specified amounts are zero, appropriate fallbacks are used.
     *
     * @param _quoteAmount The amount of the quote token requested in return on the destination chain. If set to 0, the amount specified in the configuration is used.
     */
    function transfer(uint256 _quoteAmount) external onlyProcessor {
        // Retrieve the current configuration into a local variable.
        UnionTransferConfig memory _config = config;

        // Conver the transfer token into an address.
        address transferTokenAddress = address(bytes20(_config.transferToken));

        // Check the token balance of the input account.
        uint256 balance = IERC20(transferTokenAddress).balanceOf(address(_config.inputAccount));
        if (balance == 0) {
            revert("Nothing to transfer");
        }
        if (_config.amount > balance) {
            revert("Insufficient balance");
        }

        // If the _quoteAmount provided is greater than 0, use that value; otherwise, use the configured quote amount.
        uint256 quoteAmount = _quoteAmount > 0 ? _quoteAmount : _config.quoteTokenAmount;

        // If amount is greater than 0, use that value; otherwise, transfer the full balance.
        uint256 amount = _config.amount > 0 ? _config.amount : balance;

        // if the quoteAmount is 0, set it to the amount, which means that the user wants to receive the same amount of tokens (no fees being paid).
        if (quoteAmount == 0) {
            quoteAmount = amount;
        }

        // Create the encoded operand
        bytes memory encodedOperand = createEncodedOperand(_config, amount, quoteAmount);

        // Encode the instruction to be sent to the zkGM.
        IUnion.Instruction memory instruction = IUnion.Instruction({
            version: _config.protocolVersion,
            opcode: 3, // Opcode for transferring tokens (FungibleAssetOrder)
            operand: encodedOperand
        });

        // Encode the approval call: this allows the zkGM to spend the tokens.
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (address(_config.zkGM), amount));

        // Create a unique salt using sender, timestamp and counter.
        bytes32 salt = keccak256(abi.encodePacked(msg.sender, block.timestamp, counter++));
        // Encode the send call
        bytes memory encodedSendCall = abi.encodeCall(
            IUnion.send,
            (
                _config.channelId,
                0,
                uint64((block.timestamp + _config.timeout) * 1e9), // Timeout is in nanoseconds
                salt,
                instruction
            )
        );

        // Execute the approval call on the input account.
        _config.inputAccount.execute(transferTokenAddress, 0, encodedApproveCall);
        // Execute the token send call via the zkGM.
        _config.inputAccount.execute(address(_config.zkGM), 0, encodedSendCall);
    }

    /**
     * @notice Creates an encoded operand for the zkGM instruction
     * @dev This function manually encodes all fields that would normally be in a FungibleAssetOrder struct
     * without using the struct itself. This approach is necessary because:
     * 1. The zkGM protocol expects the raw encoded fields without the additional type information
     *    that would be included when encoding a full struct with abi.encode(struct)
     * 2. Directly encoding a struct would add a 32-byte prefix indicating it's a complex object,
     *    which the protocol doesn't expect
     *
     * Note that the order of fields must exactly match what the protocol expects (like the FungibleAssetOrder in IUnion):
     * - sender (bytes from address)
     * - receiver (bytes)
     * - baseToken (bytes)
     * - baseAmount (uint256)
     * - baseTokenName (string)
     * - baseTokenSymbol (string)
     * - baseTokenDecimals (uint8)
     * - baseTokenPath (uint256)
     * - quoteToken (bytes)
     * - quoteAmount (uint256)
     *
     * @param _config The transfer configuration containing most required fields
     * @param amount The amount of tokens to transfer
     * @param quoteAmount The quote amount for the transfer
     * @return The encoded operand bytes ready to be included in the zkGM instruction
     */
    function createEncodedOperand(UnionTransferConfig memory _config, uint256 amount, uint256 quoteAmount)
        private
        pure
        returns (bytes memory)
    {
        return abi.encode(
            abi.encodePacked(address(_config.inputAccount)),
            _config.recipient,
            _config.transferToken,
            amount,
            _config.transferTokenSymbol,
            _config.transferTokenName,
            _config.transferTokenDecimals,
            _config.transferTokenUnwrappingPath,
            _config.quoteToken,
            quoteAmount
        );
    }
}
