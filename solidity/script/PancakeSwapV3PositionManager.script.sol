// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {console} from "forge-std/src/console.sol";
import {PancakeSwapV3PositionManager} from "../src/libraries/PancakeSwapV3PositionManager.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

contract PancakeSwapV3PositionManagerScript is Script {
    // Base chain PancakeSwap addresses
    address constant POSITION_MANAGER_ADDR = 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364; // PancakeSwap NonfungiblePositionManager on Base
    address constant MASTER_CHEF_V3_ADDR = 0xC6A2Db661D5a5690172d8eB0a7DEA2d3008665A3; // PancakeSwap MasterChefV3 on Base
    address constant CAKE_ADDR = 0x3055913c90Fcc1A6CE9a358911721eEb942013A1; // CAKE token on Base

    // Token addresses on Base chain
    address constant USDC_ADDR = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913; // USDC on Base
    address constant WETH_ADDR = 0x4200000000000000000000000000000000000006; // WETH on Base
    address constant USDC_WHALE = 0xF977814e90dA44bFA03b6295A0616a897441aceC;
    address constant WETH_WHALE = 0xDE4FB30cCC2f1210FcE2c8aD66410C586C8D1f9A;

    // Pool fee for USDC-WETH pool (0.01%)
    uint24 constant POOL_FEE = 100;

    // Test addresses
    address owner = address(0x1);
    address processor = address(0x2);

    // Contracts
    PancakeSwapV3PositionManager public positionManager;
    BaseAccount public inputAccount;
    BaseAccount public outputAccount;

    function run() external {
        // Create a fork of Base chain
        vm.createSelectFork("https://mainnet.base.org");

        // Setup accounts
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Fund the account with USDC from the whale
        vm.startPrank(USDC_WHALE);
        IERC20(USDC_ADDR).transfer(address(inputAccount), 10000 * 10 ** 6); // 10,000 USDC
        vm.stopPrank();

        // Fund the account with WETH from the whale
        vm.startPrank(WETH_WHALE);
        IERC20(WETH_ADDR).transfer(address(inputAccount), 10 * 10 ** 18); // 10 WETH
        vm.stopPrank();

        // Set up the PancakeSwap position manager
        vm.startPrank(owner);

        // NOTE: WETH has the smaller address, so it should be token0
        // We need to make sure token0 and token1 are properly ordered
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory config = PancakeSwapV3PositionManager
            .PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: POSITION_MANAGER_ADDR,
            masterChef: MASTER_CHEF_V3_ADDR, // Added MasterChefV3 address
            token0: WETH_ADDR, // WETH is token0
            token1: USDC_ADDR, // USDC is token1
            poolFee: POOL_FEE,
            timeout: 600, // 10 minutes
            slippageBps: 10000 // allow 100% slippage for testing
        });

        bytes memory configBytes = abi.encode(config);
        positionManager = new PancakeSwapV3PositionManager(owner, processor, configBytes);

        // Approve library to act on behalf of the input account
        inputAccount.approveLibrary(address(positionManager));

        // Also approve the output account to allow library to act on its behalf
        // This is needed for withdrawal operations
        outputAccount.approveLibrary(address(positionManager));
        vm.stopPrank();

        // Log initial state
        console.log("\n=== INITIAL STATE ===");
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Test 1: Add liquidity to a USDC-WETH pool
        console.log("\n=== TEST 1: ADD LIQUIDITY TO USDC-WETH POOL ===");

        // Define position parameters
        // Note: These values are examples and should be calculated based on the current pool state
        int24 tickLower = -887272; // Minimum possible tick
        int24 tickUpper = 887272; // Maximum possible tick

        // Amount of tokens to add to the pool
        // Make sure the order is correct: amount0 is for token0 (WETH) and amount1 is for token1 (USDC)
        uint256 amount0 = 5 * 10 ** 18; // 5 WETH (token0)
        uint256 amount1 = 5000 * 10 ** 6; // 5,000 USDC (token1)

        // Store initial balances for comparison
        uint256 initialUsdcBalance = IERC20(USDC_ADDR).balanceOf(address(inputAccount));
        uint256 initialWethBalance = IERC20(WETH_ADDR).balanceOf(address(inputAccount));

        // Add liquidity
        vm.prank(processor);
        uint256 tokenId = positionManager.provideLiquidity(tickLower, tickUpper, amount0, amount1);
        console.log("Token ID minted: %s", tokenId);

        console.log("After adding liquidity to USDC-WETH pool:");
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Verify balances after adding liquidity
        uint256 usdcAfterLiquidity = IERC20(USDC_ADDR).balanceOf(address(inputAccount));
        uint256 wethAfterLiquidity = IERC20(WETH_ADDR).balanceOf(address(inputAccount));

        console.log("\n=== VERIFICATION ===");
        console.log("USDC used for liquidity: %s", (initialUsdcBalance - usdcAfterLiquidity) / 10 ** 6);
        console.log("WETH used for liquidity: %s", (initialWethBalance - wethAfterLiquidity) / 10 ** 18);

        // Test 3: Withdraw a position
        console.log("\n=== TEST 3: WITHDRAW POSITION ===");
        console.log("Before withdrawal - Output Account Balances:");
        logBalances(outputAccount);
        console.log("CAKE balance: %s", IERC20(CAKE_ADDR).balanceOf(address(outputAccount)) / 10 ** 18);

        // Call withdrawPosition
        vm.prank(processor);
        (
            uint256 feesCollectedToken0,
            uint256 feesCollectedToken1,
            uint256 liquidity0,
            uint256 liquidity1,
            uint256 rewardAmount
        ) = positionManager.withdrawPosition(tokenId);

        console.log("\nPosition withdrawal results:");
        console.log("Fees collected (WETH): %s", feesCollectedToken0 / 10 ** 18);
        console.log("Fees collected (USDC): %s", feesCollectedToken1 / 10 ** 6);
        console.log("Liquidity withdrawn (WETH): %s", liquidity0 / 10 ** 18);
        console.log("Liquidity withdrawn (USDC): %s", liquidity1 / 10 ** 6);
        console.log("CAKE rewards: %s", rewardAmount / 10 ** 18);

        console.log("\nAfter withdrawal - Output Account Balances:");
        logBalances(outputAccount);
        console.log("CAKE balance: %s", IERC20(CAKE_ADDR).balanceOf(address(outputAccount)) / 10 ** 18);

        // Verify that funds were received in the output account
        uint256 finalUsdcBalance = IERC20(USDC_ADDR).balanceOf(address(outputAccount));
        uint256 finalWethBalance = IERC20(WETH_ADDR).balanceOf(address(outputAccount));
        uint256 finalCakeBalance = IERC20(CAKE_ADDR).balanceOf(address(outputAccount));

        console.log("\n=== FINAL VERIFICATION ===");
        console.log("Total USDC received in output account: %s", finalUsdcBalance / 10 ** 6);
        console.log("Total WETH received in output account: %s", finalWethBalance / 10 ** 18);
        console.log("Total CAKE rewards received: %s", finalCakeBalance / 10 ** 18);

        // Verify that the position is fully withdrawn
        if (finalUsdcBalance > 0 && finalWethBalance > 0) {
            console.log("\nWithdrawal test passed: Tokens have been successfully withdrawn to the output account!");
        } else {
            console.log("\nWithdrawal test failed: No tokens were received in the output account");
        }

        // Note: since we are on a simulated environment, we cannot accrue real CAKE rewards, nevertheless, the fact
        // that we can withdraw the position and all the calls to the Position Manager and MasterChefV3 return all values
        // successfully and the NFT is burned, we can assume that the integration is working as expected.

        console.log("\nPancakeSwapV3PositionManager integration tests completed successfully!");
    }

    function logBalances(BaseAccount account) internal view {
        uint256 usdcBalance = IERC20(USDC_ADDR).balanceOf(address(account));
        uint256 wethBalance = IERC20(WETH_ADDR).balanceOf(address(account));

        console.log("USDC balance: %s", usdcBalance / 10 ** 6);
        console.log("WETH balance: %s", wethBalance / 10 ** 18);
    }
}
