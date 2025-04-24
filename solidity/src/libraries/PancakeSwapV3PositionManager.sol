// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {INonfungiblePositionManager} from "./interfaces/pancakeswap/INonfungiblePositionManager.sol";
import {IMasterChefV3} from "./interfaces/pancakeswap/IMasterChefV3.sol";

/**
 * @title PancakeSwapV3PositionManager
 * @notice Contract for managing PancakeSwap V3 liquidity positions
 * @dev Handles creation, staking in MasterChef, and withdrawal of PancakeSwap V3 positions
 */
contract PancakeSwapV3PositionManager is Library {
    /**
     * @notice Configuration parameters for the PancakeSwapV3PositionManager
     * @param inputAccount Account used to provide liquidity and manage positions
     * @param outputAccount Account that receives withdrawn funds and rewards
     * @param positionManager Address of PancakeSwap's NonfungiblePositionManager contract
     * @param masterChef Address of PancakeSwap's MasterChefV3 for staking NFT positions and accrue CAKE rewards
     * @param token0 Address of the first token in the pair
     * @param token1 Address of the second token in the pair
     * @param poolFeeBps Fee tier of the liquidity pool (e.g., 500 = 0.05%)
     * @param timeout Maximum time for transactions to be valid
     * @param slippageBps Maximum allowed slippage in basis points (1 basis point = 0.01%)
     */
    struct PancakeSwapV3PositionManagerConfig {
        BaseAccount inputAccount;
        BaseAccount outputAccount;
        address positionManager;
        address masterChef;
        address token0;
        address token1;
        uint24 poolFeeBps;
        uint16 slippageBps; // Basis points (e.g., 100 = 1%)
        uint256 timeout;
    }

    /**
     * @notice Current configuration of the position manager
     */
    PancakeSwapV3PositionManagerConfig public config;

    /**
     * @notice Emitted when a position is created and a new NFT position is minted
     * @param tokenId ID of the minted NFT position
     * @param amount0Used Amount of token0 used for create position
     * @param amount1Used Amount of token1 used for create position
     */
    event PositionCreated(uint256 tokenId, uint256 amount0Used, uint256 amount1Used);

    /**
     * @notice Emitted when a position is withdrawn
     * @param tokenId ID of the withdrawn NFT position
     * @param amount0Received Amount of token0 received from the position
     * @param amount1Received Amount of token1 received from the position
     * @param rewardAmount Amount of CAKE rewards received
     */
    event PositionWithdrawn(uint256 tokenId, uint256 amount0Received, uint256 amount1Received, uint256 rewardAmount);

    /**
     * @notice Initializes the PancakeSwapV3PositionManager contract
     * @param _owner Address of the contract owner
     * @param _processor Address authorized to call the contract's functions
     * @param _config Initial configuration bytes
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the configuration parameters
     * @param _config Configuration bytes to validate
     * @return Validated configuration struct
     * @dev Checks for zero addresses and valid parameter ranges
     */
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

        // Ensure the master chef is valid
        if (decodedConfig.masterChef == address(0)) {
            revert("Master chef address can't be zero address");
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
        if (decodedConfig.poolFeeBps == 0) {
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

    /**
     * @notice Updates the contract configuration
     * @param _config New configuration bytes
     * @dev Can only be called by the contract owner
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    /**
     * @notice Creates a position on a PancakeSwap V3 pool and stakes it in MasterChef
     * @param tickLower Lower tick boundary of the position
     * @param tickUpper Upper tick boundary of the position
     * @param amount0 Amount of token0 to provide (0 = use entire balance)
     * @param amount1 Amount of token1 to provide (0 = use entire balance)
     * @return tokenId ID of the minted NFT position
     * @dev Approves tokens, mints position, and stakes in MasterChef
     */
    function createPosition(int24 tickLower, int24 tickUpper, uint256 amount0, uint256 amount1)
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
            fee: _config.poolFeeBps,
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
        emit PositionCreated(tokenId, amount0Used, amount1Used);

        return tokenId;
    }

    /**
     * @notice Withdraws a position from MasterChef, collects fees and rewards, removes liquidity
     * @param tokenId ID of the position to withdraw
     * @return feesCollectedToken0 Amount of token0 fees collected
     * @return feesCollectedToken1 Amount of token1 fees collected
     * @return liquidity0 Amount of token0 from liquidating the position
     * @return liquidity1 Amount of token1 from liquidating the position
     * @return rewardAmount Amount of CAKE rewards received
     * @dev Performs a complete withdrawal: collects fees, harvests rewards,
     *      unstakes the NFT, removes liquidity, and burns the NFT
     */
    function withdrawPosition(uint256 tokenId)
        external
        onlyProcessor
        returns (
            uint256 feesCollectedToken0,
            uint256 feesCollectedToken1,
            uint256 liquidity0,
            uint256 liquidity1,
            uint256 rewardAmount
        )
    {
        // Get the config
        PancakeSwapV3PositionManagerConfig memory _config = config;

        // Step 1: First, collect any trading fees that have accrued for the position
        // We need to call the MasterChef's collect function to get these fees
        IMasterChefV3.CollectParams memory collectFeesParams = IMasterChefV3.CollectParams({
            tokenId: tokenId,
            recipient: address(_config.outputAccount), // Send directly to output account
            amount0Max: type(uint128).max, // Collect all token0 fees
            amount1Max: type(uint128).max // Collect all token1 fees
        });

        // Encode the collectTo call - using collectTo sends the tokens directly to our specified address
        bytes memory encodedCollectToCall =
            abi.encodeCall(IMasterChefV3.collectTo, (collectFeesParams, address(_config.outputAccount)));

        // Execute the collect call to get the trading fees
        bytes memory collectFeesResult = _config.inputAccount.execute(_config.masterChef, 0, encodedCollectToCall);

        // Decode the result to get the amounts of fees collected
        (uint256 feesCollected0, uint256 feesCollected1) = abi.decode(collectFeesResult, (uint256, uint256));

        // Step 2: Now harvest all CAKE rewards and send them directly to the output account
        bytes memory encodedHarvestCall =
            abi.encodeCall(IMasterChefV3.harvest, (tokenId, address(_config.outputAccount)));

        // Execute the harvest call and capture the reward amount
        bytes memory harvestResult = _config.inputAccount.execute(_config.masterChef, 0, encodedHarvestCall);

        // Decode the harvest result to get the reward amount
        rewardAmount = abi.decode(harvestResult, (uint256));

        // Step 3: Withdraw the NFT from MasterChef to the input account
        bytes memory encodedWithdrawCall =
            abi.encodeCall(IMasterChefV3.withdraw, (tokenId, address(_config.inputAccount)));

        // Execute the withdraw call
        _config.inputAccount.execute(_config.masterChef, 0, encodedWithdrawCall);

        // Step 4: Get position details to determine how much liquidity to remove
        bytes memory encodedPositionsCall = abi.encodeCall(INonfungiblePositionManager.positions, (tokenId));

        bytes memory positionDetails = _config.inputAccount.execute(_config.positionManager, 0, encodedPositionsCall);

        // Decode the position details to get the liquidity amount and tokensOwed
        (,,,,,,, uint128 liquidity,,,,) = abi.decode(
            positionDetails,
            (uint96, address, address, address, uint24, int24, int24, uint128, uint256, uint256, uint128, uint128)
        );

        // Step 5: Remove all liquidity from the position if there is any
        if (liquidity > 0) {
            INonfungiblePositionManager.DecreaseLiquidityParams memory decreaseParams = INonfungiblePositionManager
                .DecreaseLiquidityParams({
                tokenId: tokenId,
                liquidity: liquidity,
                amount0Min: 0,
                amount1Min: 0,
                deadline: block.timestamp + _config.timeout
            });

            // Encode the decrease liquidity call
            bytes memory encodedDecreaseLiquidityCall =
                abi.encodeCall(INonfungiblePositionManager.decreaseLiquidity, (decreaseParams));

            // Execute the decrease liquidity call
            _config.inputAccount.execute(_config.positionManager, 0, encodedDecreaseLiquidityCall);
        }

        // Step 6: Collect the tokens from the position after liquidity removal
        // This is necessary because decreaseLiquidity doesn't transfer tokens, it just updates tokensOwed amounts
        INonfungiblePositionManager.CollectParams memory collectParams = INonfungiblePositionManager.CollectParams({
            tokenId: tokenId,
            recipient: address(_config.outputAccount), // Send directly to output account
            amount0Max: type(uint128).max, // Collect all token0
            amount1Max: type(uint128).max // Collect all token1
        });

        // Encode the collect call
        bytes memory encodedCollectCall = abi.encodeCall(INonfungiblePositionManager.collect, (collectParams));

        // Execute the collect call
        bytes memory collectResult = _config.inputAccount.execute(_config.positionManager, 0, encodedCollectCall);

        // Decode the result to get the actual amounts of tokens collected
        (uint256 collected0, uint256 collected1) = abi.decode(collectResult, (uint256, uint256));

        // Step 7: Burn the empty NFT position
        // This is a cleanup step and also verifies that everything has been claimed
        // (the burn will revert if liquidity or tokensOwed are not zero)
        bytes memory encodedBurnCall = abi.encodeCall(INonfungiblePositionManager.burn, (tokenId));

        _config.inputAccount.execute(_config.positionManager, 0, encodedBurnCall);

        // Emit event for the withdrawn position
        emit PositionWithdrawn(tokenId, collected0, collected1, rewardAmount);

        return (feesCollected0, feesCollected1, collected0, collected1, rewardAmount);
    }
}
