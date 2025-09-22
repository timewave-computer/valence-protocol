// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IERC4626} from "forge-std/src/interfaces/IERC4626.sol";
import {IMetaMorphoV1_1} from "./interfaces/morpho/IMetaMorphoV1_1.sol";

/**
 * @title MorphoVaultV1PositionManager
 * @dev Contract for managing Morpho Vault V1.1 through deposit and withdraw operations.
 * It leverages BaseAccount contract to interact with the Morpho Vault V1.1 protocol, enabling automated position management.
 */
contract MorphoVaultV1PositionManager is Library {
    /**
     * @title MorphoVaultV1PositionManagerConfig
     * @notice Configuration struct for Morpho Vault V1.1 Position Manager
     * @dev Used to define parameters for interacting with Morpho Vault V1.1 protocol
     * @param inputAccount The Base Account from which transactions will be initiated
     * @param outputAccount The Base Account that will receive withdrawals
     * @param vaultAddress Address of the Morpho Vault V1
     * @param assetAddress Address of the underlying asset to manage
     */
    struct MorphoVaultV1PositionManagerConfig {
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address vaultAddress;
        address assetAddress;
    }

    /// @notice Holds the current configuration for the MorphoVaultV1PositionManager.
    MorphoVaultV1PositionManagerConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the processor that can execute functions.
     * @param _config Encoded configuration parameters for the MorphoVaultV1PositionManager.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Deposits assets to MorphoVaultV1.
     * @param amount The amount to deposit (0 for all available balance).
     */
    function deposit(uint256 amount) external onlyProcessor {
        MorphoVaultV1PositionManagerConfig memory storedConfig = config;
        IERC20 asset = IERC20(storedConfig.assetAddress);

        uint256 depositAmount = amount;
        if (amount == 0) {
            depositAmount = asset.balanceOf(address(storedConfig.inputAccount));
        }

        require(depositAmount > 0, "No assets to deposit");

        //Approve the MorphoVaultV1 to spend the base asset from the input account
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (storedConfig.vaultAddress, depositAmount));

        storedConfig.inputAccount.execute(address(asset), 0, encodedApproveCall);

        // Deposit the base asset to the MorphoVaultV1
        bytes memory encodedDepositCall =
            abi.encodeCall(IERC4626.deposit, (depositAmount, address(storedConfig.inputAccount)));

        storedConfig.inputAccount.execute(storedConfig.vaultAddress, 0, encodedDepositCall);
    }

    /**
     * @notice Withdraws assets from MorphoVaultV1 position.
     * @param amount The amount to withdraw (0 to withdraw the entire balance).
     */
    function withdraw(uint256 amount) external onlyProcessor {
        MorphoVaultV1PositionManagerConfig memory storedConfig = config;
        IMetaMorphoV1_1 vault = IMetaMorphoV1_1(storedConfig.vaultAddress);

        uint256 withdrawAmount = amount;
        if (amount == 0) {
            withdrawAmount = vault.maxWithdraw(address(storedConfig.inputAccount));
        }

        require(withdrawAmount > 0, "No vault tokens to withdraw");

        // Withdraw from MorphoVaultV1 to output account
        bytes memory encodedWithdrawCall = abi.encodeCall(
            IERC4626.withdraw, (withdrawAmount, address(storedConfig.outputAccount), address(storedConfig.inputAccount))
        );

        storedConfig.inputAccount.execute(storedConfig.vaultAddress, 0, encodedWithdrawCall);
    }

    /**
     * @dev Internal initialization function called during construction
     * @param _config New configuration
     */
    function _initConfig(bytes memory _config) internal override {
        config = validateConfig(_config);
    }

    /**
     * @dev Updates the MorphoVaultV1PositionManager configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }

    /**
     * @notice Validates the provided configuration parameters
     * @dev Checks for validity of input account, output account, base asset, and market proxy address
     * @param _config The encoded configuration bytes to validate
     * @return MorphoVaultV1PositionManagerConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal view returns (MorphoVaultV1PositionManagerConfig memory) {
        // Decode the configuration bytes into the MorphoVaultV1PositionManagerConfig struct.
        MorphoVaultV1PositionManagerConfig memory decodedConfig =
            abi.decode(_config, (MorphoVaultV1PositionManagerConfig));

        // Ensure the Vault address is valid (non-zero).
        if (decodedConfig.vaultAddress == address(0)) {
            revert("Vault address can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid (non-zero).
        if (decodedConfig.outputAccount == BaseAccount(payable(address(0)))) {
            revert("Output account can't be zero address");
        }

        // Ensure the asset address is the same as the asset address of the vault
        if (decodedConfig.assetAddress != IMetaMorphoV1_1(decodedConfig.vaultAddress).asset()) {
            revert("Vault asset and given asset are not same");
        }

        return decodedConfig;
    }

    function balance() external view returns (uint256) {
        return IMetaMorphoV1_1(config.vaultAddress).previewRedeem(
            IMetaMorphoV1_1(config.vaultAddress).balanceOf(address(config.inputAccount))
        );
    }
}
