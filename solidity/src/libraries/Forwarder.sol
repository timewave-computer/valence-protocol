// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

/**
 * @title Forwarder
 * @dev Contract for automatically forwarding tokens between accounts based on configurable intervals
 */
contract Forwarder is Library {
    /**
     * @dev Configuration for a single token forwarding rule
     * @param tokenAddress Address of token to forward (0x0 for native coin)
     * @param maxAmount Maximum amount to forward per execution
     */
    struct ForwardingConfig {
        address tokenAddress;
        uint256 maxAmount;
    }

    /**
     * @dev Interval type for forwarding: time-based or block-based
     */
    enum IntervalType {
        TIME,
        BLOCKS
    }

    /**
     * @dev Main configuration struct
     * @param inputAccount Source account
     * @param outputAccount Destination account
     * @param forwardingConfigs Array of token forwarding rules
     * @param intervalType Whether to use time or block intervals
     * @param minInterval Minimum interval between forwards
     */
    struct ForwarderConfig {
        Account inputAccount;
        Account outputAccount;
        ForwardingConfig[] forwardingConfigs;
        IntervalType intervalType;
        uint64 minInterval;
    }

    /**
     * @dev Tracks last execution time/block
     */
    struct LastExecution {
        uint64 blockHeight;
        uint64 timestamp;
    }

    LastExecution public lastExecution;
    ForwarderConfig public config;

    /**
     * @dev Constructor initializes the forwarder with owner, processor, and initial configuration
     * @param _owner The initial owner of the contract
     * @param _processor The initial processor address
     * @param _config Initial configuration data for the forwarder
     * @notice Calls updateConfig to set initial forwarder configuration
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration, checking for duplicate tokens
     * @param _config Raw configuration bytes
     * @return Decoded and validated config
     */
    function validateConfig(bytes memory _config) internal pure returns (ForwarderConfig memory) {
        ForwarderConfig memory decodedConfig = abi.decode(_config, (ForwarderConfig));
        uint256 len = decodedConfig.forwardingConfigs.length;
        if (len == 0) {
            revert("No forwarding configs");
        }
        for (uint8 i = 0; i < len - 1; i++) {
            address tokenA = decodedConfig.forwardingConfigs[i].tokenAddress;
            for (uint8 j = i + 1; j < len; j++) {
                if (tokenA == decodedConfig.forwardingConfigs[j].tokenAddress) {
                    revert("Duplicate token");
                }
            }
        }
        return decodedConfig;
    }

    /**
     * @dev Updates forwarder configuration
     * @param _config New configuration
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    /**
     * @dev Main forwarding function
     * Checks interval, updates execution time, and forwards tokens
     */
    function forward() external onlyProcessor {
        _checkInterval();
        _updateLastExecution();
        Account input = config.inputAccount;
        Account output = config.outputAccount;

        for (uint8 i = 0; i < config.forwardingConfigs.length; i++) {
            ForwardingConfig memory fConfig = config.forwardingConfigs[i];
            _forwardToken(fConfig, input, output);
        }
    }

    /**
     * @dev Verifies minimum interval has passed
     */
    function _checkInterval() private view {
        if (config.intervalType == IntervalType.TIME) {
            require(block.timestamp - lastExecution.timestamp >= config.minInterval, "Time interval not passed");
        } else {
            require(block.number - lastExecution.blockHeight >= config.minInterval, "Block interval not passed");
        }
    }

    /**
     * @dev Updates last execution time/block
     */
    function _updateLastExecution() private {
        lastExecution = LastExecution(uint64(block.number), uint64(block.timestamp));
    }

    /**
     * @dev Handles forwarding for a single token
     * @param fConfig Token forwarding configuration
     * @param input Source account
     * @param output Destination account
     */
    function _forwardToken(ForwardingConfig memory fConfig, Account input, Account output) private {
        // Check if what we are trying to forward is the native coin or ERC20
        bool isNativeCoin = _isNativeCoin(fConfig.tokenAddress);

        // Determine the balance based on token type (native coin or ERC20)
        uint256 balance = isNativeCoin
            ? address(input).balance // Balance of native coin (e.g. ETH)
            : IERC20(fConfig.tokenAddress).balanceOf(address(input)); // Balance of ERC20 token

        // Calculate amount to send, capped by max amount configuration
        uint256 amountToSend = balance < fConfig.maxAmount ? balance : fConfig.maxAmount;

        // Skip execution if no amount to send
        if (amountToSend == 0) return;

        // Prepare transfer data based on token type
        bytes memory data = isNativeCoin
            ? bytes("") // No data for native coin transfer
            : abi.encodeCall(IERC20.transfer, (address(output), amountToSend)); // ERC20 transfer call data

        input.execute(
            isNativeCoin ? payable(output) : fConfig.tokenAddress, // Target: output address for Native coin, token contract for ERC20
            isNativeCoin ? amountToSend : 0, // Value: amount for Native coin, 0 for ERC20
            data // Empty for Native coin, transfer data for ERC20
        );
    }

    /**
     * @dev Checks if the given address represents the native coin (e.g. ETH)
     * @param tokenAddress Address to check
     * @return bool True if address is zero address (native coin), false otherwise
     */
    function _isNativeCoin(address tokenAddress) private pure returns (bool) {
        return tokenAddress == address(0);
    }
}
