// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {CometMainInterface} from "./interfaces/compoundV3/CometMainInterface.sol";

/**
 * @title CompoundV3PositionManager
 * @dev Contract for managing Compound V3 lending positions through supply, withdraw operations.
 * It leverages Account contracts to interact with the Compound V3 protocol, enabling automated position management.
 */
contract CompoundV3PositionManager is Library {
    /**
     * @title CompoundV3PositionManagerConfig
     * @notice Configuration struct for CompoundV3 lending operations
     * @dev Used to define parameters for interacting with CompoundV3 protocol
     * @param inputAccount The Base Account from which transactions will be initiated
     * @param outputAccount The Base Account that will receive withdrawals.
     * @param baseAsset Address of the base token of the CompoundV3 market
     * @param marketProxyAddress Address of the CompoundV3 market proxy
     */
    struct CompoundV3PositionManagerConfig {
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address baseAsset;
        address marketProxyAddress;
    }

    /// @notice Holds the current configuration for the CompoundV3PositionManager.
    CompoundV3PositionManagerConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the processor that can execute functions.
     * @param _config Encoded configuration parameters for the CompoundV3PositionManager.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the provided configuration parameters
     * @dev Checks for validity of input account, output account, base asset, and market proxy address
     * @param _config The encoded configuration bytes to validate
     * @return CompoundV3PositionManagerConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal view returns (CompoundV3PositionManagerConfig memory) {
        // Decode the configuration bytes into the CompoundV3PositionManagerConfig struct.
        CompoundV3PositionManagerConfig memory decodedConfig = abi.decode(_config, (CompoundV3PositionManagerConfig));

        // Ensure the Compound pool address is valid (non-zero).
        if (decodedConfig.marketProxyAddress == address(0)) {
            revert("Market proxy address can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid (non-zero).
        if (decodedConfig.outputAccount == BaseAccount(payable(address(0)))) {
            revert("Output account can't be zero address");
        }

        // Ensure the base asset is the same as the base asset of the market proxy
        if (decodedConfig.baseAsset != CometMainInterface(decodedConfig.marketProxyAddress).baseToken()) {
            revert("Market base asset and given base asset are not same");
        }

        return decodedConfig;
    }

    /**
     * @notice Supplies base token to the Compound V3 market and receives cUSDC (collateral tokens) in the inputAccount.
     * @dev Only the designated processor can execute this function.
     * The inputAccount must hold the base token (e.g., USDC) to supply.
     * If amount is 0, the entire balance of the base token in the inputAccount will be supplied.
     * @param amount The amount of base token to supply, or 0 to use the entire balance.
     */
    function supply(uint256 amount) external onlyProcessor {
        _supply(config.baseAsset, amount);
    }

    /**
     * @notice Supplies collateral tokens to the Compound V3 market.
     * @dev Only the designated processor can execute this function.
     * The inputAccount must hold the collateral token (e.g., wETH) to supply.
     * If amount is 0, the entire balance of the collateral token in the inputAccount will be supplied.
     * @param asset The asset to supply
     * @param amount The amount of collateral token to supply, or 0 to use the entire balance.
     *
     */
    function supplyCollateral(address asset, uint256 amount) external onlyProcessor {
        _supply(asset, amount);
    }

    function _supply(address asset, uint256 amount) internal {
        CompoundV3PositionManagerConfig memory storedConfig = config;

        uint256 amountToSupply = amount == 0 ? IERC20(asset).balanceOf(address(storedConfig.inputAccount)) : amount;

        //Approve the Compound market to spend the base asset from the input account
        bytes memory encodedApproveCall =
            abi.encodeCall(IERC20.approve, (storedConfig.marketProxyAddress, amountToSupply));

        storedConfig.inputAccount.execute(asset, 0, encodedApproveCall);

        // Supply the base asset to the Compound V3 market
        bytes memory encodedSupplyCall = abi.encodeCall(CometMainInterface.supply, (asset, amountToSupply));

        storedConfig.inputAccount.execute(storedConfig.marketProxyAddress, 0, encodedSupplyCall);
    }

    /**
     * @notice Withdraws a specified amount of base asset from the Compound V3 market to the output account.
     * @dev Only the designated processor can execute this function.
     * @param amount The amount of base asset to withdraw, or 0 to withdraw the entire balance.
     */
    function withdraw(uint256 amount) external onlyProcessor {
        _withdraw(config.baseAsset, amount);
    }

    /**
     * @notice Withdraws a specified amount of specified asset from the Compound V3 market to the output account.
     * @dev Only the designated processor can execute this function.
     * @param asset The asset to withdraw
     * @param amount The amount of base asset to withdraw, or 0 to withdraw the entire balance.
     */
    function withdrawCollateral(address asset, uint256 amount) external onlyProcessor {
        _withdraw(asset, amount);
    }

    function _withdraw(address asset, uint256 amount) internal {
        CompoundV3PositionManagerConfig memory storedConfig = config;

        // // get the withdrawable amount of base asset from the market
        uint256 amountToWithdraw = amount == 0 ? type(uint256).max : amount;

        bytes memory encodedWithdrawCall = abi.encodeCall(
            CometMainInterface.withdrawTo, (address(storedConfig.outputAccount), asset, amountToWithdraw)
        );

        storedConfig.inputAccount.execute(storedConfig.marketProxyAddress, 0, encodedWithdrawCall);
    }

    /**
     * @dev Internal initialization function called during construction
     * @param _config New configuration
     */
    function _initConfig(bytes memory _config) internal override {
        config = validateConfig(_config);
    }

    /**
     * @dev Updates the CompoundV3PositionManager configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }
}
