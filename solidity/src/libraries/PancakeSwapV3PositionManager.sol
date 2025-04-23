// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {INonfungiblePositionManager} from "./interfaces/pancakeswap/INonfungiblePositionManager.sol";

contract PancakeSwapV3PositionManager is Library {
    struct PancakeSwapV3PositionManagerConfig {
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address positionManager;
        address masterChef;
        address token0;
        address token1;
        uint24 poolFee;
        uint256 timeout;
        uint256 slippageBps; // Basis points (e.g., 100 = 1%)
    }

    PancakeSwapV3PositionManagerConfig public config;

    // Event to emit when liquidity is provided and a new NFT position is minted
    event LiquidityProvided(uint256 tokenId, uint256 amount0Used, uint256 amount1Used);

    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    function validateConfig(bytes memory _config) internal pure returns (PancakeSwapV3PositionManagerConfig memory) {
        // Decode the configuration bytes into the PancakeSwapV3PositionManagerConfig struct
        PancakeSwapV3PositionManagerConfig memory decodedConfig =
            abi.decode(_config, (PancakeSwapV3PositionManagerConfig));

        // Ensure the input account address is valid
        if (decodedConfig.inputAccount == BaseAccount(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        // Ensure the output account address is valid
        if (decodedConfig.outputAccount == BaseAccount(payable(address(0)))) {
            revert("Output account can't be zero address");
        }

        // Ensure the position manager is valid
        if (decodedConfig.positionManager == address(0)) {
            revert("Position manager address can't be zero address");
        }

        // Ensure the token0 address is valid
        if (decodedConfig.token0 == address(0)) {
            revert("Token0 address can't be zero address");
        }

        // Ensure the token1 address is valid
        if (decodedConfig.token1 == address(0)) {
            revert("Token1 address can't be zero address");
        }

        // Ensure the pool fee is valid
        if (decodedConfig.poolFee == 0) {
            revert("Pool fee can't be zero");
        }

        // Ensure the timeout is valid
        if (decodedConfig.timeout == 0) {
            revert("Timeout can't be zero");
        }

        // Ensure the slippage is valid, cant be more than 100%
        if (decodedConfig.slippageBps > 10000) {
            revert("Slippage can't be more than 100%");
        }

        return decodedConfig;
    }

    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    function provideLiquidity(int24 tickLower, int24 tickUpper, uint256 amount0, uint256 amount1)
        external
        onlyProcessor
        returns (uint256)
    {
        // Get the config
        PancakeSwapV3PositionManagerConfig memory _config = config;

        // Check if balance of assets is zero
        uint256 balanceToken0 = IERC20(address(_config.token0)).balanceOf(address(_config.inputAccount));
        if (balanceToken0 == 0) {
            revert("No token0 balance available");
        }

        uint256 balanceToken1 = IERC20(address(_config.token1)).balanceOf(address(_config.inputAccount));
        if (balanceToken1 == 0) {
            revert("No token1 balance available");
        }

        // If amounts passed are 0, we take the entire balance, otherwise we check that amounts are valid
        if (amount0 == 0) {
            amount0 = balanceToken0;
        } else if (amount0 > balanceToken0) {
            revert("Amount0 exceeds balance of token0");
        }

        if (amount1 == 0) {
            amount1 = balanceToken1;
        } else if (amount1 > balanceToken1) {
            revert("Amount1 exceeds balance of token1");
        }

        // Let's create the MintParams
        INonfungiblePositionManager.MintParams memory params = INonfungiblePositionManager.MintParams({
            token0: _config.token0,
            token1: _config.token1,
            fee: _config.poolFee,
            tickLower: tickLower,
            tickUpper: tickUpper,
            amount0Desired: amount0,
            amount1Desired: amount1,
            // Calculate minimums based on slippage
            // For 1% slippage (100 bps), we multiply by (10000 - 100) / 10000 = 0.99
            amount0Min: amount0 * (10000 - _config.slippageBps) / 10000,
            amount1Min: amount1 * (10000 - _config.slippageBps) / 10000,
            recipient: address(_config.inputAccount),
            deadline: block.timestamp + _config.timeout
        });

        // Encode the approval call for the tokens: this allows the Position Manager to spend the tokens.
        bytes memory encodedApproveCallToken0 =
            abi.encodeCall(IERC20.approve, (address(_config.positionManager), amount0));
        bytes memory encodedApproveCallToken1 =
            abi.encodeCall(IERC20.approve, (address(_config.positionManager), amount1));

        // Encode the mint call
        bytes memory encodedMintCall = abi.encodeCall(INonfungiblePositionManager.mint, (params));

        // Execute the calls
        _config.inputAccount.execute(_config.token0, 0, encodedApproveCallToken0);
        _config.inputAccount.execute(_config.token1, 0, encodedApproveCallToken1);
        // Execute the mint call and extract the tokenId
        bytes memory result = _config.inputAccount.execute(_config.positionManager, 0, encodedMintCall);

        // Extract tokenId, liquidity, amount0, amount1 from the result
        (uint256 tokenId,, uint256 amount0Used, uint256 amount1Used) =
            abi.decode(result, (uint256, uint128, uint256, uint256));

        // We are going to now stake the position by transferring the NFT to the masterChef
        // Encode the transfer call
        bytes memory encodedTransferCall = abi.encodeWithSignature(
            "safeTransferFrom(address,address,uint256,bytes)",
            address(_config.inputAccount),
            address(_config.masterChef),
            tokenId,
            "" // Empty bytes for the data parameter
        );
        // Execute the transfer call
        _config.inputAccount.execute(address(_config.positionManager), 0, encodedTransferCall);

        // Emit event
        emit LiquidityProvided(tokenId, amount0Used, amount1Used);

        return tokenId;
    }
}
