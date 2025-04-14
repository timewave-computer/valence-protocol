// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IStandardBridge} from "./interfaces/standard-bridge/IStandardBridge.sol";

/**
 * @title StandardBridgeTransfer
 * @dev Contract for automatically transferring tokens using StandardBridge on both L1 and L2.
 * Works with both L1StandardBridge and L2StandardBridge contracts.
 * It allows for transferring ETH or ERC20 tokens across chains.
 */
contract StandardBridgeTransfer is Library {
    /**
     * @dev Configuration struct for StandardBridge transfer parameters.
     * @param amount The number of tokens to transfer. If set to 0, the entire balance is transferred.
     * @param inputAccount The account from which tokens will be transferred from.
     * @param recipient The recipient address on the destination chain.
     * @param standardBridge The StandardBridge contract address (L1 or L2 version).
     * @param token The ERC20 token address to transfer (or address(0) for ETH).
     * @param remoteToken Address of the corresponding token on the destination chain (for ERC20).
     * @param minGasLimit Gas to use to complete the transfer on the receiving side. Used for sequencers/relayers.
     * @param extraData Additional data to be forwarded with the transaction.
     */
    struct StandardBridgeTransferConfig {
        uint256 amount;
        BaseAccount inputAccount;
        address recipient;
        IStandardBridge standardBridge;
        address token;
        address remoteToken;
        uint32 minGasLimit;
        bytes extraData;
    }

    // Holds the current configuration for StandardBridge transfers
    StandardBridgeTransferConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the StandardBridge transfer.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration by decoding the provided bytes and ensuring no critical addresses are zero.
     * @param _config Raw configuration bytes.
     * @return Decoded and validated StandardBridgeTransferConfig struct.
     */
    function validateConfig(bytes memory _config) internal pure returns (StandardBridgeTransferConfig memory) {
        // Decode the configuration bytes into the StandardBridgeTransferConfig struct.
        StandardBridgeTransferConfig memory decodedConfig = abi.decode(_config, (StandardBridgeTransferConfig));

        // Ensure the StandardBridge is a valid (non-zero) address.
        if (decodedConfig.standardBridge == IStandardBridge(payable(address(0)))) {
            revert("StandardBridge can't be zero address");
        }

        // Ensure the recipient address is valid (non-zero).
        if (decodedConfig.recipient == address(0)) {
            revert("Recipient can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // For ERC20 transfers, ensure remote token is specified
        if (decodedConfig.token != address(0) && decodedConfig.remoteToken == address(0)) {
            revert("Remote token must be specified for ERC20 transfers");
        }

        return decodedConfig;
    }

    /**
     * @dev Updates the StandardBridgeTransfer configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @dev Executes the token or ETH transfer using the appropriate StandardBridge functions.
     * Works for both L1->L2 and L2->L1.
     */
    function transfer() external onlyProcessor {
        // Retrieve the current configuration into a local variable.
        StandardBridgeTransferConfig memory _config = config;

        // Check if we're transferring ETH or ERC20
        bool isETH = _config.token == address(0);

        // Get the current balance
        uint256 balance;
        if (isETH) {
            balance = address(_config.inputAccount).balance;
        } else {
            balance = IERC20(_config.token).balanceOf(address(_config.inputAccount));
        }

        // Check balance
        if (balance == 0) {
            revert("No balance to transfer");
        } else if (_config.amount > 0 && balance < _config.amount) {
            revert("Insufficient balance");
        }

        // Determine the amount to transfer
        uint256 _amount = _config.amount > 0 ? _config.amount : balance;

        if (isETH) {
            // ETH transfer using bridgeETHTo
            bytes memory bridgeETHCall =
                abi.encodeCall(IStandardBridge.bridgeETHTo, (_config.recipient, _config.minGasLimit, _config.extraData));

            // Execute the ETH bridge call
            _config.inputAccount.execute(address(_config.standardBridge), _amount, bridgeETHCall);
        } else {
            // ERC20 transfer
            // First approve the bridge to spend tokens
            bytes memory approveCall = abi.encodeCall(IERC20.approve, (address(_config.standardBridge), _amount));

            // Then bridge the tokens using bridgeERC20To
            bytes memory bridgeERC20Call = abi.encodeCall(
                IStandardBridge.bridgeERC20To,
                (_config.token, _config.remoteToken, _config.recipient, _amount, _config.minGasLimit, _config.extraData)
            );

            // Execute the approval and bridge calls
            _config.inputAccount.execute(_config.token, 0, approveCall);
            _config.inputAccount.execute(address(_config.standardBridge), 0, bridgeERC20Call);
        }
    }
}
