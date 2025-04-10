// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {console} from "forge-std/src/console.sol";
import {BalancerV2Swap} from "../src/libraries/BalancerV2Swap.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {IAsset, IBalancerVault} from "../src/libraries/interfaces/balancerV2/IBalancerVault.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {MockERC20} from "../test/mocks/MockERC20.sol";

contract BalancerV2SwapScript is Script {
    // Berachain Balancer addresses
    address constant BALANCER_VAULT_ADDR = 0x4Be03f781C497A489E3cB0287833452cA9B9E80B;

    // Token addresses on Berachain
    address constant HONEY_ADDR = 0xFCBD14DC51f0A4d49d5E53C2E0950e0bC26d0Dce;
    address constant USDCE_ADDR = 0x549943e04f40284185054145c6E4e9568C1D3241;
    address constant BYUSD_ADDR = 0x688e72142674041f8f6Af4c808a4045cA1D6aC82;

    // Pool IDs
    bytes32 constant HONEY_USDCE_POOL_ID = 0xf961a8f6d8c69e7321e78d254ecafbcc3a637621000000000000000000000001;
    bytes32 constant HONEY_BYUSD_POOL_ID = 0xde04c469ad658163e2a5e860a03a86b52f6fa8c8000000000000000000000000;

    // Test addresses
    address owner = address(0x1);
    address processor = address(0x2);

    // Contracts
    BalancerV2Swap public balancerSwap;
    BaseAccount public inputAccount;
    BaseAccount public outputAccount;

    function run() external {
        // Create a fork of Berachain
        vm.createSelectFork("https://rpc.berachain.com");

        // Setup accounts
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Replace the runtime code at USDCE_ADDR with our MockERC20 code so we can mint some USDC to the BaseAccount
        bytes memory mockCode = type(MockERC20).runtimeCode;
        vm.etch(USDCE_ADDR, mockCode);

        // Mint some USDC tokens to the BaseAccount
        MockERC20 usdc = MockERC20(USDCE_ADDR);
        usdc.mint(address(inputAccount), 1000 * 10 ** 6); // Mint 1000 USDCe

        // Set up the Balancer swap manager
        vm.startPrank(owner);
        BalancerV2Swap.BalancerV2SwapConfig memory config = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            vaultAddress: BALANCER_VAULT_ADDR
        });

        bytes memory configBytes = abi.encode(config);
        balancerSwap = new BalancerV2Swap(owner, processor, configBytes);

        // Approve library to act on behalf of the input account
        inputAccount.approveLibrary(address(balancerSwap));
        vm.stopPrank();

        // Set amounts for testing
        uint256 swapAmount = 100 * 10 ** 6; // 100 USDCe
        uint256 minAmountOut = 1; // Expecting at least 1 token
        uint256 timeout = 600; // 10 minutes

        // Store initial balances for comparison
        uint256 initialUsdceBalance = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));
        uint256 initialOutHoneyBalance = IERC20(HONEY_ADDR).balanceOf(address(outputAccount));
        uint256 initialOutByusdBalance = IERC20(BYUSD_ADDR).balanceOf(address(outputAccount));

        // Log initial state
        console.log("\n=== INITIAL STATE ===");
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Verify initial setup
        require(initialUsdceBalance >= swapAmount, "Insufficient USDCE for testing");

        // TEST 1: SINGLE SWAP (USDCE -> HONEY)
        console.log("\n=== TEST 1: SINGLE SWAP (USDCE -> HONEY) ===");
        vm.prank(processor);
        balancerSwap.swap(
            HONEY_USDCE_POOL_ID,
            IAsset(USDCE_ADDR),
            IAsset(HONEY_ADDR),
            bytes(""), // userData (usually empty for simple swaps)
            swapAmount,
            minAmountOut,
            timeout
        );

        console.log("After swapping %s USDCE for HONEY:", swapAmount / 10 ** 6);
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Verify balances after single swap
        uint256 usdceAfterSwap = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));
        uint256 honeyAfterSwap = IERC20(HONEY_ADDR).balanceOf(address(outputAccount));

        require(
            usdceAfterSwap == initialUsdceBalance - swapAmount, "USDCE balance didn't decrease correctly after swap"
        );

        require(honeyAfterSwap > initialOutHoneyBalance, "HONEY balance didn't increase after swap");

        // TEST 2: MULTI-HOP SWAP (USDCE -> HONEY -> BYUSD)
        console.log("\n=== TEST 2: MULTI-HOP SWAP (USDCE -> HONEY -> BYUSD) ===");

        // Setup for multi-hop swap
        uint256 multiSwapAmount = 100 * 10 ** 6; // 100 USDCe
        uint256 multiMinAmountOut = 1 * 10 ** 6; // Expecting at least 1 BYUSD

        // Prepare arrays for multi-hop swap
        bytes32[] memory poolIds = new bytes32[](2);
        poolIds[0] = HONEY_USDCE_POOL_ID; // First pool: USDCE -> HONEY
        poolIds[1] = HONEY_BYUSD_POOL_ID; // Second pool: HONEY -> BYUSD

        IAsset[] memory tokens = new IAsset[](3);
        tokens[0] = IAsset(USDCE_ADDR); // Starting token: USDCE
        tokens[1] = IAsset(HONEY_ADDR); // Intermediate token: HONEY
        tokens[2] = IAsset(BYUSD_ADDR); // Final token: BYUSD

        bytes[] memory userDataArray = new bytes[](2);
        userDataArray[0] = bytes("");
        userDataArray[1] = bytes("");

        // Store balances before multi-hop swap
        uint256 usdceBeforeMulti = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));
        uint256 byusdBeforeMulti = IERC20(BYUSD_ADDR).balanceOf(address(outputAccount));

        vm.prank(processor);
        balancerSwap.multiSwap(poolIds, tokens, userDataArray, multiSwapAmount, multiMinAmountOut, timeout);

        console.log("After multi-hop swapping %s USDCE -> HONEY -> BYUSD:", multiSwapAmount / 10 ** 6);
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Verify balances after multi-hop swap
        uint256 usdceAfterMulti = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));
        uint256 byusdAfterMulti = IERC20(BYUSD_ADDR).balanceOf(address(outputAccount));

        require(
            usdceAfterMulti == usdceBeforeMulti - multiSwapAmount,
            "USDCE balance didn't decrease correctly after multi-hop swap"
        );

        require(byusdAfterMulti > byusdBeforeMulti, "BYUSD balance didn't increase after multi-hop swap");

        // TEST 3: SWAP ALL (using amount = 0)
        console.log("\n=== TEST 3: SWAP ALL REMAINING USDCE ===");

        vm.prank(processor);
        balancerSwap.swap(
            HONEY_USDCE_POOL_ID,
            IAsset(USDCE_ADDR),
            IAsset(HONEY_ADDR),
            bytes(""),
            0, // 0 means swap all available balance
            0, // No minimum for simplicity
            timeout
        );

        console.log("After swapping ALL remaining USDCE for HONEY:");
        console.log("Input Account Balances:");
        logBalances(inputAccount);
        console.log("\nOutput Account Balances:");
        logBalances(outputAccount);

        // Verify all USDCE was swapped
        uint256 usdceAfterSwapAll = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));

        require(usdceAfterSwapAll == 0, "Not all USDCE was swapped when using amount = 0");

        // Final verification
        console.log("\n=== FINAL VERIFICATION ===");

        uint256 finalInUsdceBalance = IERC20(USDCE_ADDR).balanceOf(address(inputAccount));
        uint256 finalOutHoneyBalance = IERC20(HONEY_ADDR).balanceOf(address(outputAccount));
        uint256 finalOutByusdBalance = IERC20(BYUSD_ADDR).balanceOf(address(outputAccount));

        if (finalInUsdceBalance == 0) {
            console.log("Swap tests passed: All USDCE has been swapped!");
        } else {
            console.log("Not all USDCE was swapped. Remaining: %s", finalInUsdceBalance / 10 ** 6);
            revert("Swap tests failed: Not all USDCE was swapped");
        }

        if (finalOutHoneyBalance > initialOutHoneyBalance) {
            console.log("Swap tests passed: Output account received HONEY!");
            console.log("Total HONEY gained: %s", (finalOutHoneyBalance - initialOutHoneyBalance) / 10 ** 18);
        } else {
            revert("Swap tests failed: Output account didn't receive HONEY");
        }

        if (finalOutByusdBalance > initialOutByusdBalance) {
            console.log("Multi-hop swap tests passed: Output account received BYUSD!");
            console.log("Total BYUSD gained: %s", (finalOutByusdBalance - initialOutByusdBalance) / 10 ** 6);
        } else {
            revert("Multi-hop swap tests failed: Output account didn't receive BYUSD");
        }

        console.log("\nBalancerV2Swap integration tests completed successfully!");
    }

    function logBalances(BaseAccount account) internal view {
        uint256 honeyBalance = IERC20(HONEY_ADDR).balanceOf(address(account));
        uint256 usdceBalance = IERC20(USDCE_ADDR).balanceOf(address(account));
        uint256 byusdBalance = IERC20(BYUSD_ADDR).balanceOf(address(account));

        console.log("HONEY balance: %s", honeyBalance / 10 ** 18);
        console.log("USDCE balance: %s", usdceBalance / 10 ** 6);
        console.log("BYUSD balance: %s", byusdBalance / 10 ** 6);
    }
}
