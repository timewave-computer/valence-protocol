// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IAsset, IBalancerVault} from "./interfaces/balancerV2/IBalancerVault.sol";

/**
 * @title BalancerV2Swap
 * @dev Contract for performing token swaps using the Balancer V2 protocol.
 * It leverages the central Balancer Vault to execute both single and multi-hop swaps.
 * All swaps are configured as GIVEN_IN swaps, where the input amount is specified
 * and the output amount is calculated by the protocol.
 */
contract BalancerV2Swap is Library {
    /**
     * @title BalancerV2SwapConfig
     * @notice Configuration for Balancer V2 swaps
     * @param inputAccount The account from which tokens will be taken
     * @param outputAccount The account to which result tokens will be sent
     * @param vaultAddress Address of the Balancer V2 Vault
     */
    struct BalancerV2SwapConfig {
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address vaultAddress;
    }

    /// @notice Base configuration for the BalancerV2Swap
    BalancerV2SwapConfig public config;

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the designated processor that can execute functions.
     * @param _config Encoded configuration parameters for the Balancer swap.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the provided configuration parameters
     * @param _config The encoded configuration bytes to validate
     * @return BalancerV2SwapConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal pure returns (BalancerV2SwapConfig memory) {
        // Decode the configuration bytes into the BalancerV2SwapConfig struct
        BalancerV2SwapConfig memory decodedConfig = abi.decode(_config, (BalancerV2SwapConfig));

        // Ensure the input account address is valid
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid
        if (decodedConfig.outputAccount == BaseAccount(payable(address(0)))) {
            revert("Output account can't be zero address");
        }

        // Ensure the vault address is valid
        if (decodedConfig.vaultAddress == address(0)) {
            revert("Vault address can't be zero address");
        }

        return decodedConfig;
    }

    /**
     * @dev Internal initialization function called during construction
     * @param _config New configuration
     */
    function _initConfig(bytes memory _config) internal override {
        config = validateConfig(_config);
    }

    /**
     * @dev Updates the BalancerV2Swap configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    /**
     * @notice Validates the single swap parameters
     * @param poolId The ID of the Balancer pool to use for the swap
     * @param tokenIn Address of the token to swap from
     * @param tokenOut Address of the token to swap to
     * @param timeout How long the transaction is valid for (in seconds)
     */
    function validateSingleSwapParams(bytes32 poolId, IAsset tokenIn, IAsset tokenOut, uint256 timeout) internal pure {
        if (poolId == bytes32(0)) {
            revert("Pool ID can't be empty for single swap");
        }

        if (address(tokenIn) == address(0)) {
            revert("Token in can't be zero address for single swap");
        }

        if (address(tokenOut) == address(0)) {
            revert("Token out can't be zero address for single swap");
        }

        if (address(tokenIn) == address(tokenOut)) {
            revert("Token in and out can't be the same");
        }

        if (timeout == 0) {
            revert("Timeout can't be zero");
        }
    }

    /**
     * @notice Validates the multi-hop swap parameters
     * @param poolIds Array of pool IDs to use for each swap step
     * @param tokens Array of all tokens involved in the swap path (in sequence)
     * @param userDataArray Additional data for specialized pools
     * @param timeout How long the transaction is valid for (in seconds)
     */
    function validateMultiSwapParams(
        bytes32[] calldata poolIds,
        IAsset[] calldata tokens,
        bytes[] calldata userDataArray,
        uint256 timeout
    ) internal pure {
        if (poolIds.length == 0) {
            revert("Pool IDs array can't be empty for multi-hop swap");
        }

        if (tokens.length == 0) {
            revert("Tokens array can't be empty for multi-hop swap");
        }

        // For multi-hop swaps, Tokens array must have at least one more element than poolIds
        if (tokens.length != poolIds.length + 1) {
            revert("Tokens array must contain at least poolIds.length + 1 elements");
        }

        // Validate userData array length if provided
        if (userDataArray.length != poolIds.length) {
            revert("userData array length must match poolIds length");
        }

        // Validate each pool ID is not empty (bytes32(0))
        for (uint256 i = 0; i < poolIds.length; i++) {
            if (poolIds[i] == bytes32(0)) {
                revert("Pool ID can't be empty in poolIds array");
            }
        }

        // Validate each token is not a zero address
        for (uint256 i = 0; i < tokens.length; i++) {
            if (address(tokens[i]) == address(0)) {
                revert("Token can't be zero address in tokens array");
            }
        }

        if (timeout == 0) {
            revert("Timeout can't be zero");
        }
    }

    /**
     * @notice Executes a single swap through Balancer V2
     * @param poolId The ID of the Balancer pool to use for the swap
     * @param tokenIn Address of the token to swap from
     * @param tokenOut Address of the token to swap to
     * @param userData Additional data for specialized pools (usually empty bytes)
     * @param amountIn Amount of tokens to swap. If set to 0, all available tokens will be swapped
     * @param minAmountOut Minimum amount of output tokens to receive (slippage protection)
     * @param timeout How long the transaction is valid for (in seconds)
     */
    function swap(
        bytes32 poolId,
        IAsset tokenIn,
        IAsset tokenOut,
        bytes memory userData,
        uint256 amountIn,
        uint256 minAmountOut,
        uint256 timeout
    ) external onlyProcessor {
        // Get the config
        BalancerV2SwapConfig memory swapConfig = config;

        // Validate single swap parameters
        validateSingleSwapParams(poolId, tokenIn, tokenOut, timeout);

        // Get the current balance of tokenIn asset in the input account
        uint256 balance = IERC20(address(tokenIn)).balanceOf(address(swapConfig.inputAccount));

        // Check if balance is zero
        if (balance == 0) {
            revert("No asset balance available");
        }

        // If amountIn is 0, use the entire balance
        uint256 amountToSwap = amountIn == 0 ? balance : amountIn;

        // Check if there's enough balance for the requested amount
        if (balance < amountToSwap) {
            revert("Insufficient asset balance");
        }

        // Create the SingleSwap struct
        IBalancerVault.SingleSwap memory singleSwap = IBalancerVault.SingleSwap({
            poolId: poolId,
            kind: IBalancerVault.SwapKind.GIVEN_IN,
            assetIn: tokenIn,
            assetOut: tokenOut,
            amount: amountToSwap,
            userData: userData
        });

        // Create the FundManagement struct
        IBalancerVault.FundManagement memory funds = IBalancerVault.FundManagement({
            sender: address(swapConfig.inputAccount),
            fromInternalBalance: false,
            recipient: payable(address(swapConfig.outputAccount)),
            toInternalBalance: false
        });

        // Set deadline
        uint256 deadline = block.timestamp + timeout;

        // First, approve the Vault to spend tokens from the input account
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (swapConfig.vaultAddress, amountToSwap));
        swapConfig.inputAccount.execute(address(tokenIn), 0, encodedApproveCall);

        // Then execute the swap
        bytes memory encodedSwapCall = abi.encodeCall(IBalancerVault.swap, (singleSwap, funds, minAmountOut, deadline));

        // Execute the swap from the input account
        swapConfig.inputAccount.execute(swapConfig.vaultAddress, 0, encodedSwapCall);
    }

    /**
     * @notice Executes a multi-hop swap through Balancer V2
     * @param poolIds Array of pool IDs to use for each swap step
     * @param tokens Array of all tokens involved in the swap path (in sequence)
     * @param userDataArray Additional data for specialized pools (usually empty bytes for each step)
     * @param amountIn Amount of tokens to swap. If set to 0, all available tokens will be swapped
     * @param minAmountOut Minimum amount of output tokens to receive (slippage protection)
     * @param timeout How long the transaction is valid for (in seconds)
     */
    function multiSwap(
        bytes32[] calldata poolIds,
        IAsset[] calldata tokens,
        bytes[] calldata userDataArray,
        uint256 amountIn,
        uint256 minAmountOut,
        uint256 timeout
    ) external onlyProcessor {
        // Get the config
        BalancerV2SwapConfig memory swapConfig = config;

        // Validate multi-hop swap parameters
        validateMultiSwapParams(poolIds, tokens, userDataArray, timeout);

        // Get the initial token (first asset in the path)
        IAsset initialToken = tokens[0];

        // Get the current balance of tokenIn asset in the input account
        uint256 balance = IERC20(address(initialToken)).balanceOf(address(swapConfig.inputAccount));

        // Check if balance is zero
        if (balance == 0) {
            revert("No asset balance available");
        }

        // If amountIn is 0, use the entire balance
        uint256 amountToSwap = amountIn == 0 ? balance : amountIn;

        // Check if there's enough balance for the requested amount
        if (balance < amountToSwap) {
            revert("Insufficient asset balance");
        }

        // Create the BatchSwapStep array
        IBalancerVault.BatchSwapStep[] memory swapSteps = new IBalancerVault.BatchSwapStep[](poolIds.length);
        for (uint256 i = 0; i < poolIds.length; i++) {
            // For the first step, use the specified amount; for subsequent steps, use 0 (amount flows through)
            uint256 stepAmount = i == 0 ? amountToSwap : 0;

            swapSteps[i] = IBalancerVault.BatchSwapStep({
                poolId: poolIds[i],
                assetInIndex: i, // Asset index for input token (sequential)
                assetOutIndex: i + 1, // Asset index for output token (sequential)
                amount: stepAmount,
                userData: userDataArray.length > i ? userDataArray[i] : bytes("")
            });
        }

        // Create the FundManagement struct
        IBalancerVault.FundManagement memory funds = IBalancerVault.FundManagement({
            sender: address(swapConfig.inputAccount),
            fromInternalBalance: false,
            recipient: payable(address(swapConfig.outputAccount)),
            toInternalBalance: false
        });

        // Set deadline
        uint256 deadline = block.timestamp + timeout;

        // Create limits array for multi-hop swap
        // For more info on limits for batch swaps, check https://docs-v2.balancer.fi/reference/swaps/batch-swaps.html#batchswap-function
        int256[] memory limits = new int256[](tokens.length);

        // Set input token limit (maximum to spend) - Positive value indicates Balancer this is the maximum amount to spend.
        limits[0] = int256(amountToSwap);

        // Set output token limit (minimum to receive) - Negative value indicates Balancer this is the minimum amount to receive.
        limits[tokens.length - 1] = -1 * int256(minAmountOut);

        // All other limits remain 0 (no restrictions on intermediate tokens)

        // First, approve the Vault to spend tokens from the input account
        bytes memory encodedApproveCall = abi.encodeCall(IERC20.approve, (swapConfig.vaultAddress, amountToSwap));
        swapConfig.inputAccount.execute(address(initialToken), 0, encodedApproveCall);

        // Then execute the batch swap
        bytes memory encodedBatchSwapCall = abi.encodeCall(
            IBalancerVault.batchSwap, (IBalancerVault.SwapKind.GIVEN_IN, swapSteps, tokens, funds, limits, deadline)
        );

        // Execute the batch swap from the input account
        swapConfig.inputAccount.execute(swapConfig.vaultAddress, 0, encodedBatchSwapCall);
    }
}
