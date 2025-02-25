// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {ITokenMessenger} from "./interfaces/cctp/ITokenMessenger.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

/**
 * @title CCTPTransfer
 * @dev Contract for automatically transferring tokens using the CCTP protocol.
 * It leverages an external CCTP Token Messenger contract to handle cross-chain token transfers.
 */
contract CCTPTransfer is Library {
    /**
     * @dev Configuration struct for token transfer parameters.
     * @param amount The number of tokens to transfer. If set to 0, the entire balance is transferred.
     * @param mintRecipient The recipient address (in bytes32 format) on the destination chain where tokens will be minted.
     * @param inputAccount The account from which tokens will be debited.
     * @param destinationDomain The domain identifier for the destination chain.
     * @param cctpTokenMessenger The CCTP Token Messenger contract.
     * @param transferToken The ERC20 token address that will be transferred.
     */
    struct CCTPTransferConfig {
        uint256 amount; // If we want to transfer all tokens, we can set this to 0.
        bytes32 mintRecipient;
        Account inputAccount;
        uint32 destinationDomain;
        ITokenMessenger cctpTokenMessenger;
        address transferToken;
    }

    // Holds the current configuration for token transfers
    CCTPTransferConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the CCTP transfer.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration by decoding the provided bytes and ensuring no critical addresses are zero.
     * This prevents misconfiguration.
     * @param _config Raw configuration bytes.
     * @return Decoded and validated CCTPTransferConfig struct.
     */
    function validateConfig(bytes memory _config) internal pure returns (CCTPTransferConfig memory) {
        // Decode the configuration bytes into the CCTPTransferConfig struct.
        CCTPTransferConfig memory decodedConfig = abi.decode(_config, (CCTPTransferConfig));

        // Ensure the CCTP Token Messenger is a valid (non-zero) address.
        if (decodedConfig.cctpTokenMessenger == ITokenMessenger(address(0))) {
            revert("CCTP Token Messenger can't be zero address");
        }

        // Ensure the transfer token address is valid (non-zero).
        if (decodedConfig.transferToken == address(0)) {
            revert("Transfer token can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == Account(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        return decodedConfig;
    }

    /**
     * @dev Updates the CCTPTransfer configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @dev Executes the token transfer using the CCTP protocol.
     *
     * Steps:
     * 1. Retrieve the current configuration.
     * 2. Check the token balance of the input account to ensure sufficient funds.
     * 3. Determine the transfer amount; if set to 0, use the full balance.
     * 4. Approve the CCTP Token Messenger to spend the tokens from the input account.
     * 5. Encode and execute the depositForBurn call to transfer the tokens.
     *
     * Requirements:
     * - The caller must be the designated processor.
     * - The input account must hold enough tokens.
     */
    function transfer() external onlyProcessor {
        // Retrieve the current configuration into a local variable.
        CCTPTransferConfig memory _config = config;

        // Check the token balance of the input account.
        uint256 balance = IERC20(_config.transferToken).balanceOf(address(_config.inputAccount));
        if (balance == 0 && _config.amount == 0) {
            revert("Nothing to transfer");
        } else if (balance == 0 || balance < _config.amount) {
            revert("Insufficient balance");
        }

        // Determine the amount to transfer:
        // If amount is greater than 0, use that value; otherwise, transfer the full balance.
        uint256 _amount = _config.amount > 0 ? _config.amount : balance;

        // Encode the approval call: this allows the CCTP Token Messenger to spend the tokens.
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (address(_config.cctpTokenMessenger), _amount));

        // Encode the transfer call: deposit tokens for burning, which triggers the cross-chain transfer.
        bytes memory encodedTransferCall = abi.encodeCall(
            ITokenMessenger.depositForBurn,
            (_amount, _config.destinationDomain, _config.mintRecipient, _config.transferToken)
        );

        // Execute the approval call on the input account.
        _config.inputAccount.execute(_config.transferToken, 0, encodedApproveCall);
        // Execute the token transfer call via the CCTP Token Messenger.
        _config.inputAccount.execute(address(_config.cctpTokenMessenger), 0, encodedTransferCall);
    }
}
