// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {IPool} from "aave-v3-origin/interfaces/IPool.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

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
     * @param aavePoolAddress The address of the Aave V3 Pool contract
     * @param inputAccount The account from which transactions will be initiated
     * @param outputAccount The account that will receive aTokens, borrowed assets or withdrawals. Can be the same as inputAccount.
     * @param supplyAsset Address of the token to supply to Aave
     * @param borrowAsset Address of the token to borrow from Aave
     * @param referralCode Referral code for Aave protocol (if applicable - 0 if the action is executed directly by the user, without any middle-men)
     */
    struct AavePositionManagerConfig {
        IPool aavePoolAddress;
        Account inputAccount;
        Account outputAccount;
        address supplyAsset;
        address borrowAsset;
        uint16 referralCode;
    }

    /// @notice Holds the current configuration for the AavePositionManager.
    AavePositionManagerConfig public config;

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

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == Account(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid (non-zero).
        if (decodedConfig.outputAccount == Account(payable(address(0)))) {
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
     * The output account will receive the corresponding aTokens.
     * @param amount The amount of tokens to supply
     */
    function supply(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Encode the approval call for the Aave pool.
        bytes memory encodedApproveCall =
            abi.encodeCall(IERC20.approve, (address(storedConfig.aavePoolAddress), amount));

        // Execute the approval from the input account
        storedConfig.inputAccount.execute(storedConfig.supplyAsset, 0, encodedApproveCall);

        // Supply the specified asset to the Aave protocol.
        bytes memory encodedSupplyCall = abi.encodeCall(
            IPool.supply,
            (storedConfig.supplyAsset, amount, address(storedConfig.outputAccount), storedConfig.referralCode)
        );

        // Execute the supply from the input account
        storedConfig.inputAccount.execute(address(storedConfig.aavePoolAddress), 0, encodedSupplyCall);
    }

    /**
     * @notice Borrows tokens from the Aave protocol
     * @dev Only the designated processor can execute this function.
     * Borrows assets from Aave against the collateral previously supplied.
     * The output account will receive the borrowed tokens.
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to borrow
     */
    function borrow(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Borrow the specified asset from the Aave protocol.
        bytes memory encodedBorrowCall = abi.encodeCall(
            IPool.borrow,
            (storedConfig.borrowAsset, amount, 2, storedConfig.referralCode, address(storedConfig.outputAccount))
        );

        // Execute the borrow from the input account
        storedConfig.inputAccount.execute(address(storedConfig.aavePoolAddress), 0, encodedBorrowCall);
    }

    /**
     * @notice Withdraws previously supplied tokens from Aave
     * @dev Only the designated processor can execute this function.
     * Withdraws assets from Aave and sends them to the output account.
     * This reduces the available collateral for any outstanding loans.
     * @param amount The amount of tokens to withdraw
     */
    function withdraw(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Withdraw the specified asset from the Aave protocol.
        bytes memory encodedWithdrawCall =
            abi.encodeCall(IPool.withdraw, (storedConfig.supplyAsset, amount, address(storedConfig.outputAccount)));

        // Execute the withdraw from the input account
        storedConfig.inputAccount.execute(address(storedConfig.aavePoolAddress), 0, encodedWithdrawCall);
    }

    /**
     * @notice Repays borrowed tokens to the Aave protocol
     * @dev Only the designated processor can execute this function.
     * First approves the Aave pool to spend tokens, then repays the loan
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to repay
     */
    function repay(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Encode the approval call for the Aave pool.
        bytes memory encodedApproveCall =
            abi.encodeCall(IERC20.approve, (address(storedConfig.aavePoolAddress), amount));

        // Execute the approval from the input account
        storedConfig.inputAccount.execute(storedConfig.borrowAsset, 0, encodedApproveCall);

        // Repay the specified asset to the Aave protocol.
        bytes memory encodedRepayCall =
            abi.encodeCall(IPool.repay, (storedConfig.borrowAsset, amount, 2, address(storedConfig.inputAccount)));

        // Execute the repay from the input account
        storedConfig.inputAccount.execute(address(storedConfig.aavePoolAddress), 0, encodedRepayCall);
    }

    /**
     * @notice Repays borrowed tokens using aTokens directly
     * @dev Only the designated processor can execute this function.
     * Allows repaying loans using the interest-bearing aTokens themselves,
     * which can be more gas-efficient than converting aTokens to underlying assets first.
     * Uses interest rate mode 2 (variable rate), which is only one supported for this operation.
     * @param amount The amount of tokens to repay using aTokens
     */
    function repayWithATokens(uint256 amount) external onlyProcessor {
        // Get the current configuration.
        AavePositionManagerConfig memory storedConfig = config;

        // Repay the specified asset to the Aave protocol using aTokens.
        bytes memory encodedRepayCall = abi.encodeCall(IPool.repayWithATokens, (storedConfig.borrowAsset, amount, 2));

        // Execute the repay from the input account
        storedConfig.inputAccount.execute(address(storedConfig.aavePoolAddress), 0, encodedRepayCall);
    }

    /**
     * @dev Updates the AavePositionManager configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        // Validate and update the configuration.
        config = validateConfig(_config);
    }
}
