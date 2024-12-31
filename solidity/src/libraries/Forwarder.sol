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
        uint128 maxAmount;
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
     * @param forwarding_configs Array of token forwarding rules
     * @param intervalType Whether to use time or block intervals
     * @param minInterval Minimum interval between forwards
     */
    struct ForwarderConfig {
        Account inputAccount;
        Account outputAccount;
        ForwardingConfig[] forwarding_configs;
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
        uint256 len = decodedConfig.forwarding_configs.length;
        for (uint256 i = 0; i < len - 1; i++) {
            address tokenA = decodedConfig.forwarding_configs[i].tokenAddress;
            for (uint256 j = i + 1; j < len; j++) {
                if (tokenA == decodedConfig.forwarding_configs[j].tokenAddress) {
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

        for (uint256 i = 0; i < config.forwarding_configs.length; i++) {
            ForwardingConfig memory fConfig = config.forwarding_configs[i];
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
        uint256 balance;
        bytes memory data;

        if (fConfig.tokenAddress == address(0)) {
            // Handle native coin (ETH)
            balance = address(input).balance;
            data = "";
        } else {
            // Handle ERC20 token
            balance = IERC20(fConfig.tokenAddress).balanceOf(address(input));
            data = abi.encodeCall(
                IERC20.transfer, (address(output), balance < fConfig.maxAmount ? balance : fConfig.maxAmount)
            );
        }

        // Takes smaller of: available balance or configured max amount
        uint256 amountToSend = balance < fConfig.maxAmount ? balance : fConfig.maxAmount;

        if (amountToSend > 0) {
            input.execute(
                fConfig.tokenAddress == address(0) ? payable(output) : fConfig.tokenAddress, // Target: output address for ETH, token contract for ERC20
                fConfig.tokenAddress == address(0) ? amountToSend : 0, // Value: amount for ETH, 0 for ERC20
                data // Empty for ETH, transfer data for ERC20
            );
        }
    }
}
