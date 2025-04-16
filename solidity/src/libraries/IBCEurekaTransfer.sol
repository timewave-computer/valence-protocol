// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IEurekaHandler} from "./interfaces/eureka/IEurekaHandler.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

/**
 * @title IBCEurekaTransfer
 * @dev Contract for transferring tokens using IBC Eureka solidity implementation.
 * It leverages an external EurekaHandler contract to handle cross-chain token transfers.
 */
contract IBCEurekaTransfer is Library {
    /**
     * @dev Configuration struct for token transfer parameters.
     * @param amount The number of tokens to transfer. If set to 0, the entire balance is transferred.
     * @param transferToken The ERC20 token address that will be transferred.
     * @param inputAccount The account from which tokens will be debited.
     * @param recipient The recipient address on the destination IBC chain (in bech32 format).
     * @param sourceClient The source client identifier (e.g. cosmoshub-0).
     * @param timeout The timeout for the IBC transfer in seconds.
     * @param eurekaHandler The EurekaHandler contract which is a wrapper around the ICS20Transfer contract.
     */
    struct IBCEurekaTransferConfig {
        uint256 amount;
        address transferToken;
        BaseAccount inputAccount;
        string recipient;
        string sourceClient;
        uint64 timeout;
        IEurekaHandler eurekaHandler;
    }

    // Holds the current configuration for token transfers
    IBCEurekaTransferConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the IBC Eureka transfer.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration by decoding the provided bytes and ensuring no critical addresses are zero.
     * This prevents misconfiguration.
     * @param _config Raw configuration bytes.
     * @return Decoded and validated IBCEurekaTransferConfig struct.
     */
    function validateConfig(bytes memory _config) internal pure returns (IBCEurekaTransferConfig memory) {
        // Decode the configuration bytes into the IBCEurekaTransferConfig struct.
        IBCEurekaTransferConfig memory decodedConfig = abi.decode(_config, (IBCEurekaTransferConfig));

        // Ensure the Eureka Handler is a valid (non-zero) address.
        if (decodedConfig.eurekaHandler == IEurekaHandler(address(0))) {
            revert("Eureka Handler can't be zero address");
        }

        // Ensure the transfer token address is valid (non-zero).
        if (decodedConfig.transferToken == address(0)) {
            revert("Transfer token can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the timeout value is greater than zero.
        if (decodedConfig.timeout == 0) {
            revert("Timeout can't be zero");
        }

        return decodedConfig;
    }

    /**
     * @dev Updates the IBCEurekaTransfer configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @dev Executes the token transfer using the IBC Eureka protocol via an EurekaHandler contract.
     *
     * The function performs several key operations:
     * 1. Validates token balances and transfer amounts
     * 2. Calculates the final transfer amount after deducting relay fees
     * 3. Approves the EurekaHandler to spend tokens from the input account
     * 4. Executes the cross-chain transfer via the EurekaHandler
     *
     * @param fees The fee structure containing relay fees, recipient of the relay fees and quote expiry.
     * @param memo Additional information to be included with the transfer. Can execute logic on the destination chain. Can be empty if not required.
     *
     * Requirements:
     * - The caller must be the designated processor.
     * - The input account must hold enough tokens.
     */
    function transfer(IEurekaHandler.Fees calldata fees, string calldata memo) external onlyProcessor {
        // Retrieve the current configuration into a local variable.
        IBCEurekaTransferConfig memory _config = config;

        // Check the token balance of the input account.
        uint256 balance = IERC20(_config.transferToken).balanceOf(address(_config.inputAccount));
        if (balance == 0) {
            revert("Nothing to transfer");
        }
        if (_config.amount > balance) {
            revert("Insufficient balance");
        }

        // If amount is greater than 0, use that value; otherwise, transfer the full balance.
        uint256 amount = _config.amount > 0 ? _config.amount : balance;

        // Check that we have enough balance to cover the fees.
        if (amount <= fees.relayFee) {
            revert("Not enough to pay fees and make a transfer");
        }

        // Subtract the relay fee from the amount to be transferred.
        uint256 amountToTransfer = amount - fees.relayFee;

        // Encode the approval call: this allows the Eureka Handler to spend the tokens.
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (address(_config.eurekaHandler), amount));

        // Build the TransferParams struct for the transfer.
        IEurekaHandler.TransferParams memory transferParams = IEurekaHandler.TransferParams({
            token: _config.transferToken,
            recipient: _config.recipient,
            sourceClient: _config.sourceClient,
            destPort: "transfer",
            timeoutTimestamp: uint64(block.timestamp) + _config.timeout,
            memo: memo
        });

        // Encode the transfer call
        bytes memory encodedTransferCall =
            abi.encodeCall(IEurekaHandler.transfer, (amountToTransfer, transferParams, fees));

        // Execute the approval call on the input account.
        _config.inputAccount.execute(_config.transferToken, 0, encodedApproveCall);
        // Execute the token transfer call via the Eureka Handler.
        _config.inputAccount.execute(address(_config.eurekaHandler), 0, encodedTransferCall);
    }
}
