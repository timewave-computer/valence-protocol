// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IPool} from "aave-v3-origin/interfaces/IPool.sol";
import {AToken} from "aave-v3-origin/protocol/tokenization/AToken.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IAaveIncentivesController} from "aave-v3-origin/interfaces/IAaveIncentivesController.sol";
import {IRewardsDistributor} from "aave-v3-origin/rewards/interfaces/IRewardsDistributor.sol";
import {IRewardsController} from "aave-v3-origin/rewards/interfaces/IRewardsController.sol";

/**
 * @title AavePositionManager
 * @dev Contract for managing Aave V3 lending positions through deposit, borrow, withdraw, and repay operations.
 * It leverages Account contracts to interact with the Aave protocol, enabling automated position management.
 */
contract AavePositionManager is Library {
    /**
     * @title AavePositionManagerConfig
     * @notice Configuration struct for Aave lending operations
     * @dev Used to define parameters for interacting with Aave V3 protocol
     * @param poolAddress The address of the Aave V3 Pool contract
     * @param inputAccount The Base Account from which transactions will be initiated
     * @param outputAccount The Base Account that will receive withdrawals. Can be the same as inputAccount.
     * @param supplyAsset Address of the token to supply to Aave
     * @param borrowAsset Address of the token to borrow from Aave
     * @param referralCode Referral code for Aave protocol (if applicable - 0 if the action is executed directly by the user, without any middle-men)
     */
    struct AavePositionManagerConfig {
        IPool poolAddress;
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address supplyAsset;
        address borrowAsset;
        uint16 referralCode;
    }

    /// @notice Holds the current configuration for the AavePositionManager.
    AavePositionManagerConfig public config;

    /**
     * @title AavePositionManagerDerivedConfig
     * @notice Configuration struct derived from the AavePositionManagerConfig
     * @param rewardsController The address of the Aave Incentives Controller contract
     * @param aToken The address of the aToken issued against the supply asset
     * @param debtToken The address of the debtToken issued against the borrow asset
     */
    struct AavePositionManagerDerivedConfig {
        IAaveIncentivesController rewardsController;
        address aToken;
        address debtToken;
    }

    /// @notice Holds the derived configuration for the AavePositionManager.
    AavePositionManagerDerivedConfig public derivedConfig;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the processor that can execute functions.
     * @param _config Encoded configuration parameters for the AavePositionManager.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the provided configuration parameters
     * @dev Checks for validity of input account, output account, supply asset, and borrow asset
     * @param _config The encoded configuration bytes to validate
     * @return AavePositionManagerConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal pure returns (AavePositionManagerConfig memory) {
        // Decode the configuration bytes into the AavePositionManagerConfig struct.
        AavePositionManagerConfig memory decodedConfig = abi.decode(_config, (AavePositionManagerConfig));

        // Ensure the Aave pool address is valid (non-zero).
        if (address(decodedConfig.poolAddress) == address(0)) {
            revert("Aave pool address can't be zero address");
        }

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid (non-zero).
        if (decodedConfig.outputAccount == BaseAccount(payable(address(0)))) {
            revert("Output account can't be zero address");
        }

        // Ensure the supply asset address is valid (non-zero).
        if (decodedConfig.supplyAsset == address(0)) {
            revert("Supply asset can't be zero address");
        }

        // Ensure the borrow asset address is valid (non-zero).
        if (decodedConfig.borrowAsset == address(0)) {
            revert("Borrow asset can't be zero address");
        }

        return decodedConfig;
    }

    /**
     * @notice Supplies tokens to the Aave protocol
     * @dev Only the designated processor can execute this function.
     * First approves the Aave pool to spend tokens, then supplies them to the protocol.
     * The input account will receive the corresponding aTokens.
     * If amount is 0, the entire balance of the supply asset in the input account will be used.
     * @param amount The amount of tokens to supply, or 0 to use entire balance
     */
    function supply(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Get the current balance of the supply asset in the input account
        uint256 balance = IERC20(storedConfig.supplyAsset).balanceOf(address(storedConfig.inputAccount));

        // Check if balance is zero
        if (balance == 0) {
            revert("No supply asset balance available");
        }

        // If amount is 0, use the entire balance
        uint256 amountToSupply = amount == 0 ? balance : amount;

        // Check if there's enough balance for the requested amount
        if (balance < amountToSupply) {
            revert("Insufficient supply asset balance");
        }

        // Encode the approval call for the Aave pool.
        bytes memory encodedApproveCall =
            abi.encodeCall(IERC20.approve, (address(storedConfig.poolAddress), amountToSupply));

        // Execute the approval from the input account
        storedConfig.inputAccount.execute(storedConfig.supplyAsset, 0, encodedApproveCall);

        // Supply the specified asset to the Aave protocol.
        bytes memory encodedSupplyCall = abi.encodeCall(
            IPool.supply,
            (storedConfig.supplyAsset, amountToSupply, address(storedConfig.inputAccount), storedConfig.referralCode)
        );

        // Execute the supply from the input account
        storedConfig.inputAccount.execute(address(storedConfig.poolAddress), 0, encodedSupplyCall);
    }

    /**
     * @notice Borrows tokens from the Aave protocol
     * @dev Only the designated processor can execute this function.
     * Borrows assets from Aave against the collateral previously supplied.
     * The input account will receive the borrowed tokens.
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to borrow
     */
    function borrow(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Borrow the specified asset from the Aave protocol.
        bytes memory encodedBorrowCall = abi.encodeCall(
            IPool.borrow,
            (storedConfig.borrowAsset, amount, 2, storedConfig.referralCode, address(storedConfig.inputAccount))
        );

        // Execute the borrow from the input account
        storedConfig.inputAccount.execute(address(storedConfig.poolAddress), 0, encodedBorrowCall);
    }

    /**
     * @notice Withdraws previously supplied tokens from Aave
     * @dev Only the designated processor can execute this function.
     * Withdraws assets from Aave and sends them to the output account.
     * This reduces the available collateral for any outstanding loans.
     * @param amount The amount of tokens to withdraw, passing 0 will withdraw the entire balance
     */
    function withdraw(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // If amount is 0, use uint256.max to withdraw as much as possible
        if (amount == 0) {
            amount = type(uint256).max;
        }

        // Withdraw the specified asset from the Aave protocol.
        bytes memory encodedWithdrawCall =
            abi.encodeCall(IPool.withdraw, (storedConfig.supplyAsset, amount, address(storedConfig.outputAccount)));

        // Execute the withdraw from the input account
        storedConfig.inputAccount.execute(address(storedConfig.poolAddress), 0, encodedWithdrawCall);
    }

    /**
     * @notice Repays borrowed tokens to the Aave protocol
     * @dev Only the designated processor can execute this function.
     * First approves the Aave pool to spend tokens, then repays the loan
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to repay, if amount is 0, repays the entire balance
     */
    function repay(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Get the current balance of the borrow asset in the input account
        uint256 balance = IERC20(storedConfig.borrowAsset).balanceOf(address(storedConfig.inputAccount));

        // Check if balance is zero
        if (balance == 0) {
            revert("No borrow asset balance available");
        }

        // If amount is 0, use the entire balance
        uint256 amountToRepay = amount == 0 ? balance : amount;

        // Check if there's enough balance for the requested amount
        if (balance < amountToRepay) {
            revert("Insufficient borrow asset balance");
        }

        // Encode the approval call for the Aave pool.
        bytes memory encodedApproveCall =
            abi.encodeCall(IERC20.approve, (address(storedConfig.poolAddress), amountToRepay));

        // Execute the approval from the input account
        storedConfig.inputAccount.execute(storedConfig.borrowAsset, 0, encodedApproveCall);

        // Repay the specified asset to the Aave protocol.
        bytes memory encodedRepayCall = abi.encodeCall(
            IPool.repay, (storedConfig.borrowAsset, amountToRepay, 2, address(storedConfig.inputAccount))
        );

        // Execute the repay from the input account
        storedConfig.inputAccount.execute(address(storedConfig.poolAddress), 0, encodedRepayCall);
    }

    /**
     * @notice Repays borrowed tokens using aTokens directly
     * @dev Only the designated processor can execute this function.
     * Allows repaying loans using the interest-bearing aTokens themselves,
     * which can be more gas-efficient than converting aTokens to underlying assets first.
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to repay using aTokens, passing 0 will repay as much as possible
     */
    function repayWithShares(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // If amount is 0, use uint256.max to repay as much as possible
        if (amount == 0) {
            amount = type(uint256).max;
        }

        // Repay the specified asset to the Aave protocol using aTokens.
        bytes memory encodedRepayCall = abi.encodeCall(IPool.repayWithATokens, (storedConfig.borrowAsset, amount, 2));

        // Execute the repay from the input account
        storedConfig.inputAccount.execute(address(storedConfig.poolAddress), 0, encodedRepayCall);
    }

    function getAllRewards() external view returns (address[] memory, uint256[] memory) {
        // Get the current configuration.
        AavePositionManagerDerivedConfig memory storedDerivedConfig = derivedConfig;

        address[] memory assets = _getAssets(storedDerivedConfig);

        // Get the rewards from the Aave protocol and return it.
        return IRewardsDistributor(address(storedDerivedConfig.rewardsController)).getAllUserRewards(
            assets, address(config.inputAccount)
        );
    }

    function claimRewards(address rewardToken, uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;
        AavePositionManagerDerivedConfig memory storedDerivedConfig = derivedConfig;

        // If amount is 0, use uint256.max to claim as much as possible
        if (amount == 0) {
            amount = type(uint256).max;
        }

        // Claim the rewards from the Aave protocol.
        address[] memory assets = _getAssets(storedDerivedConfig);
        bytes memory encodedClaimRewardsCall = abi.encodeCall(
            IRewardsController.claimRewards, (assets, amount, address(storedConfig.outputAccount), rewardToken)
        );

        // Execute the claim rewards from the input account
        storedConfig.inputAccount.execute(address(storedDerivedConfig.rewardsController), 0, encodedClaimRewardsCall);
    }

    function claimAllRewards() external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;
        AavePositionManagerDerivedConfig memory storedDerivedConfig = derivedConfig;

        // Claim all rewards from the Aave protocol.
        address[] memory assets = _getAssets(storedDerivedConfig);
        bytes memory encodedClaimRewardsCall =
            abi.encodeCall(IRewardsController.claimAllRewards, (assets, address(storedConfig.outputAccount)));

        // Execute the claim rewards from the input account
        storedConfig.inputAccount.execute(address(storedDerivedConfig.rewardsController), 0, encodedClaimRewardsCall);
    }

    /**
     * @dev Internal initialization function called during construction
     * @param _config New configuration
     */
    function _initConfig(bytes memory _config) internal override {
        config = validateConfig(_config);
        _fetchDerivedConfig();
    }

    /**
     * @dev Updates the AavePositionManager configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
        _fetchDerivedConfig();
    }

    function _fetchDerivedConfig() internal {
        // Get the aToken and debtToken addresses
        address aToken = config.poolAddress.getReserveAToken(config.supplyAsset);
        address debtToken = config.poolAddress.getReserveVariableDebtToken(config.borrowAsset);

        derivedConfig = AavePositionManagerDerivedConfig({
            aToken: aToken,
            debtToken: debtToken,
            rewardsController: AToken(aToken).getIncentivesController()
        });
    }

    function _getAssets(AavePositionManagerDerivedConfig memory storedDerivedConfig)
        internal
        pure
        returns (address[] memory)
    {
        address[] memory assets = new address[](2);
        assets[0] = storedDerivedConfig.aToken;
        assets[1] = storedDerivedConfig.debtToken;
        return assets;
    }
}
