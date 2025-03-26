// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IStargate} from "@stargatefinance/stg-evm-v2/src/interfaces/IStargate.sol";
import {MessagingFee, OFTReceipt, SendParam} from "@layerzerolabs/lz-evm-oapp-v2/contracts/oft/interfaces/IOFT.sol";

/**
 * @title StargateTransfer
 * @dev Contract for automatically transferring tokens using the Stargate V2 protocol built on top of LayerZero.
 * It leverages an external StargatePool/StargatePoolUSDC contract to handle cross-chain token transfers.
 */
contract StargateTransfer is Library {
    /**
     * @title StargateConfig
     * @notice Configuration struct for cross-chain token transfers via Stargate Protocol
     * @dev Used to define parameters for LayerZero cross-chain messaging with Stargate
     * @param recipient The recipient address (in bytes32 format) on the destination chain
     * @param inputAccount The account from which tokens will be transferred
     * @param destinationDomain The destination chain endpoint ID. Find all IDs at https://stargateprotocol.gitbook.io/stargate/v2-developer-docs/technical-reference/mainnet-contracts
     * @param stargateAddress Stargate pool address implementing IOFT interface. See https://github.com/stargate-protocol/stargate-v2/blob/main/packages/stg-evm-v2/src/interfaces/IStargate.sol
     * @param transferToken Address of the token to transfer. If transferring native tokens, this will be the zero address (address(0))
     * @param amount Amount of tokens to transfer. If set to 0, all available tokens will be transferred
     * @param minAmountToReceive Minimum amount to receive on destination after fees. If set to 0, fees will be automatically calculated
     * @param refundAddress Address to refund tokens in case of failed transfer. If set to address(0), tokens will be refunded to the input account
     * @param extraOptions Additional options for the LayerZero message. Optional. See https://docs.layerzero.network/v2/developers/evm/protocol-gas-settings/options#option-types
     * @param composeMsg Message to execute logic on the destination chain. Optional. See https://docs.layerzero.network/v2/developers/evm/composer/overview#composing-an-oft--onft
     * @param oftCmd Indicates the transportation mode in Stargate. Empty bytes for "Taxi" mode, bytes(1) for "Bus" mode. See https://stargateprotocol.gitbook.io/stargate/v2-developer-docs/integrate-with-stargate/how-to-swap#sendparam.oftcmd
     */
    struct StargateConfig {
        bytes32 recipient;
        Account inputAccount;
        uint32 destinationDomain;
        IStargate stargateAddress;
        address transferToken;
        uint256 amount;
        uint256 minAmountToReceive;
        address refundAddress;
        bytes extraOptions;
        bytes composeMsg;
        bytes oftCmd;
    }

    /// @notice Holds the current configuration for token transfers
    StargateConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the Stargate transfer.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the provided configuration parameters
     * @dev Checks for validity of input account, stargate address, token match, and amount
     * @param _config The encoded configuration bytes to validate
     * @return StargateConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal view returns (StargateConfig memory) {
        // Decode the configuration bytes into the CCTPTransferConfig struct.
        StargateConfig memory decodedConfig = abi.decode(_config, (StargateConfig));

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == Account(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the starport address is valid (non-zero).
        if (decodedConfig.stargateAddress == IStargate(address(0))) {
            revert("Stargate address can't be zero address");
        }

        // Ensure the transfer token and the token address in the stargate address match.
        if (decodedConfig.stargateAddress.token() != decodedConfig.transferToken) {
            revert("Token address does not match the stargate token address");
        }

        return decodedConfig;
    }

    /**
     * @dev Updates the StargateTransfer configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @notice Initiates a cross-chain token transfer using Stargate protocol
     * @dev Only the designated processor can execute this function.
     * The function handles both native and ERC20 token transfers.
     * It calculates required fees, determines minimum receiving amounts,
     * approves tokens (if needed), and executes the transfer through the
     * associated input account.
     */
    function transfer() external onlyProcessor {
        StargateConfig memory storedConfig = config;

        // Get the native token balance of the input account.
        uint256 inputAccountNativeBalance = address(storedConfig.inputAccount).balance;

        // Determine the amount to transfer
        uint256 amountToTransfer = storedConfig.amount;

        // If amount is 0, transfer the full balance of the token
        if (amountToTransfer == 0) {
            if (storedConfig.transferToken == address(0)) {
                // For native token, use the account's balance
                amountToTransfer = inputAccountNativeBalance;
            } else {
                // For ERC20 tokens, get the balance
                amountToTransfer = IERC20(storedConfig.transferToken).balanceOf(address(storedConfig.inputAccount));
            }

            // Revert if no balance to transfer
            if (amountToTransfer == 0) {
                revert("No balance to transfer");
            }
        }

        // Create the SendParam struct
        SendParam memory sendParam = SendParam({
            dstEid: storedConfig.destinationDomain,
            to: storedConfig.recipient,
            amountLD: amountToTransfer,
            minAmountLD: storedConfig.minAmountToReceive,
            extraOptions: storedConfig.extraOptions,
            composeMsg: storedConfig.composeMsg,
            oftCmd: storedConfig.oftCmd
        });

        IStargate stargate = storedConfig.stargateAddress;

        // Calculate the fees and minimum amount to receive
        if (storedConfig.minAmountToReceive == 0) {
            // Quote the OFT to get the expected receipt amount
            (,, OFTReceipt memory receipt) = stargate.quoteOFT(sendParam);
            sendParam.minAmountLD = receipt.amountReceivedLD;
            // If there is nothing to receive, abort the transfer
            if (sendParam.minAmountLD == 0) {
                revert("Nothing to receive after fees");
            }
        }

        // Get messaging fee
        MessagingFee memory messagingFee = stargate.quoteSend(sendParam, false);
        uint256 valueToSend = messagingFee.nativeFee;

        // Set refund address
        address refundAddress = storedConfig.refundAddress;
        if (refundAddress == address(0)) {
            refundAddress = address(storedConfig.inputAccount);
        }

        // Handle differently based on token type
        if (stargate.token() == address(0)) {
            // Native token transfer

            // If sending full balance, adjust for fees
            if (amountToTransfer == inputAccountNativeBalance) {
                // Initial fee estimate
                if (amountToTransfer <= messagingFee.nativeFee) {
                    revert("Insufficient balance for fees");
                }

                // There is an edge case here when we are transfering full amounts:
                // Since the fees on Stargate need to be the exact amount, we need to adjust the amount
                // to leave room for the initial fee estimate. If we don't do this, the transfer will fail
                // This might leave some dust in the account if the recalculated fees are less, but it's necessary for the transfer to succeed

                // First iteration: adjust amount to leave room for initial fee estimate
                uint256 initialAdjustedAmount = amountToTransfer - messagingFee.nativeFee;

                // Create a temporary sendParam with adjusted amount for requoting
                SendParam memory tempSendParam = sendParam;
                tempSendParam.amountLD = initialAdjustedAmount;

                // Recalculate the messaging fee based on the adjusted amount
                MessagingFee memory recalculatedFee = stargate.quoteSend(tempSendParam, false);

                // Final adjustment: make sure we have enough for the recalculated fee
                if (amountToTransfer <= recalculatedFee.nativeFee) {
                    revert("Insufficient balance for fees after recalculation");
                }

                // Set the final adjusted amount
                sendParam.amountLD = amountToTransfer - recalculatedFee.nativeFee;

                // Recalculate minimum amount to receive if needed
                if (storedConfig.minAmountToReceive == 0) {
                    (,, OFTReceipt memory receipt) = stargate.quoteOFT(sendParam);
                    sendParam.minAmountLD = receipt.amountReceivedLD;
                    // If there is nothing to receive, abort the transfer
                    if (sendParam.minAmountLD == 0) {
                        revert("Nothing to receive after fees");
                    }
                }

                // For native tokens, send both the adjusted transfer amount plus the recalculated fee
                valueToSend = sendParam.amountLD + recalculatedFee.nativeFee;

                // Encode the sendToken call for native tokens with recalculated fee
                bytes memory encodedSendCall =
                    abi.encodeCall(IStargate.sendToken, (sendParam, recalculatedFee, refundAddress));

                // Execute from the input account with the full value (adjusted amount + recalculated fee)
                storedConfig.inputAccount.execute(address(stargate), valueToSend, encodedSendCall);
            } else {
                // Check if there's enough balance for specified amount plus fees
                if (inputAccountNativeBalance < sendParam.amountLD + messagingFee.nativeFee) {
                    revert("Insufficient balance for transfer and fees");
                }

                // For native tokens, need to send both the transfer amount and fee together
                valueToSend = sendParam.amountLD + messagingFee.nativeFee;

                // Encode the sendToken call for native tokens
                bytes memory encodedSendCall =
                    abi.encodeCall(IStargate.sendToken, (sendParam, messagingFee, refundAddress));

                // Execute from the input account
                storedConfig.inputAccount.execute(address(stargate), valueToSend, encodedSendCall);
            }
        } else {
            // ERC20 token transfer

            // Check if input account has enough native balance for fees
            if (inputAccountNativeBalance < messagingFee.nativeFee) {
                revert("Insufficient native balance for fees");
            }

            // Check if input account has enough token balance
            if (IERC20(storedConfig.transferToken).balanceOf(address(storedConfig.inputAccount)) < sendParam.amountLD) {
                revert("Insufficient token balance");
            }

            // Encode the approval call
            bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (address(stargate), sendParam.amountLD));

            // Execute the approval from the input account
            storedConfig.inputAccount.execute(storedConfig.transferToken, 0, encodedApproveCall);

            // Encode the sendToken call for ERC20 tokens
            bytes memory encodedSendCall = abi.encodeCall(IStargate.sendToken, (sendParam, messagingFee, refundAddress));

            // Execute the sendToken from the input account
            storedConfig.inputAccount.execute(address(stargate), messagingFee.nativeFee, encodedSendCall);
        }
    }
}
