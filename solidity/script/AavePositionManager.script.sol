// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {console} from "forge-std/src/console.sol";
import {AavePositionManager} from "../src/libraries/AavePositionManager.sol";
import {Account} from "../src/accounts/Account.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {IPool} from "aave-v3-origin/interfaces/IPool.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

contract AavePositionManagerScript is Script {
    // Mainnet addresses
    address constant AAVE_POOL_ADDR = 0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2;
    address constant USDC_ADDR = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant AUSDC_ADDR = 0x98C23E9d8f34FEFb1B7BD6a91B7FF122F4e16F5c;
    address constant DAI_ADDR = 0x6B175474E89094C44Da98b954EedeAC495271d0F;
    address constant ADAI_ADDR = 0x018008bfb33d285247A21d44E50697654f754e63;
    address constant VARIABLE_DEBT_DAI = 0xcF8d0c70c850859266f5C338b38F9D663181C314;

    // Let's use a whale to transfer some tokens as we need to fund the account
    // Tried replacing the runtime code of the tokens but it messed up with AAVE's calculations because tokens had different decimals
    // Binance hot wallet - huge USDC and DAI holdings
    address constant USDC_WHALE = 0x28C6c06298d514Db089934071355E5743bf21d60;
    address constant DAI_WHALE = 0x28C6c06298d514Db089934071355E5743bf21d60;

    // Test addresses
    address owner = address(0x1);
    address processor = address(0x2);

    // Contracts
    AavePositionManager public aaveManager;
    BaseAccount public account;

    function run() external {
        // Create a fork of mainnet
        vm.createSelectFork("https://eth-mainnet.public.blastapi.io");

        // Setup account
        vm.startPrank(owner);
        account = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Fund the account with USDC from the whale
        vm.startPrank(USDC_WHALE);
        IERC20(USDC_ADDR).transfer(address(account), 1_000_000 * 10 ** 6); // 1M USDC
        vm.stopPrank();

        // Fund the account with DAI from the whale
        vm.startPrank(DAI_WHALE);
        IERC20(DAI_ADDR).transfer(address(account), 1_000_000 * 10 ** 18); // 1M DAI
        vm.stopPrank();

        // Set up the Aave manager with the same account for input and output, to avoid requiring to transfer tokens afterwards
        vm.startPrank(owner);
        AavePositionManager.AavePositionManagerConfig memory config = AavePositionManager.AavePositionManagerConfig({
            aavePoolAddress: IPool(AAVE_POOL_ADDR),
            inputAccount: account,
            outputAccount: account,
            supplyAsset: USDC_ADDR,
            borrowAsset: DAI_ADDR,
            referralCode: 0
        });

        bytes memory configBytes = abi.encode(config);
        aaveManager = new AavePositionManager(owner, processor, configBytes);

        // Approve library to act on behalf of the account
        account.approveLibrary(address(aaveManager));
        vm.stopPrank();

        // Set amounts for testing
        uint256 supplyAmount = 100_000 * 10 ** 6; // 100,000 USDC
        uint256 borrowAmount = 10_000 * 10 ** 18; // 10,000 DAI
        uint256 withdrawAmount = 20_000 * 10 ** 6; // 20,000 USDC
        uint256 repayAmount = 3_000 * 10 ** 18; // 3,000 DAI

        // Store initial balances for comparison
        uint256 initialUsdcBalance = IERC20(USDC_ADDR).balanceOf(address(account));
        uint256 initialDaiBalance = IERC20(DAI_ADDR).balanceOf(address(account));

        // Log initial state
        console.log("\n=== INITIAL STATE ===");
        logBalances();

        // Verify initial setup
        require(initialUsdcBalance >= supplyAmount, "Insufficient USDC for testing");
        require(initialDaiBalance >= repayAmount, "Insufficient DAI for testing");

        // TEST 1: SUPPLY
        console.log("\n=== TEST 1: SUPPLY ===");
        vm.prank(processor);
        aaveManager.supply(supplyAmount);
        console.log("After supplying %s USDC:", supplyAmount / 10 ** 6);
        logBalances();

        // Verify that USDC balance has decreased by the correct amount
        uint256 usdcAfterSupply = IERC20(USDC_ADDR).balanceOf(address(account));
        require(
            usdcAfterSupply == initialUsdcBalance - supplyAmount, "USDC balance didn't decrease correctly after supply"
        );

        // Verify that aUSDC balance has increased
        uint256 aUsdcBalance = IERC20(AUSDC_ADDR).balanceOf(address(account));
        require(aUsdcBalance > 0, "Supply operation failed: No aUSDC received");

        // TEST 2: BORROW
        console.log("\n=== TEST 2: BORROW ===");
        vm.prank(processor);
        aaveManager.borrow(borrowAmount);
        console.log("After borrowing %s DAI:", borrowAmount / 10 ** 18);
        logBalances();

        // Verify that DAI balance increased by the borrowed amount
        uint256 daiAfterBorrow = IERC20(DAI_ADDR).balanceOf(address(account));
        require(
            daiAfterBorrow >= initialDaiBalance + borrowAmount - 10,
            "DAI balance didn't increase correctly after borrow"
        );

        // Verify that debt token increased
        uint256 debtDaiBalance = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));
        require(debtDaiBalance >= borrowAmount - 10, "Debt not registered correctly");

        // TEST 3: PARTIAL WITHDRAW
        console.log("\n=== TEST 3: PARTIAL WITHDRAW ===");
        uint256 usdcBeforeWithdraw = IERC20(USDC_ADDR).balanceOf(address(account));
        uint256 aUsdcBeforeWithdraw = IERC20(AUSDC_ADDR).balanceOf(address(account));

        vm.prank(processor);
        aaveManager.withdraw(withdrawAmount);
        console.log("After withdrawing %s USDC:", withdrawAmount / 10 ** 6);
        logBalances();

        // Verify that USDC balance increased by the withdrawn amount
        uint256 usdcAfterWithdraw = IERC20(USDC_ADDR).balanceOf(address(account));
        require(
            usdcAfterWithdraw >= usdcBeforeWithdraw + withdrawAmount - 10,
            "USDC balance didn't increase correctly after withdraw"
        );

        // Verify that aUSDC balance decreased
        uint256 aUsdcAfterWithdraw = IERC20(AUSDC_ADDR).balanceOf(address(account));
        require(aUsdcAfterWithdraw < aUsdcBeforeWithdraw, "aUSDC balance didn't decrease after withdraw");

        // TEST 4: PARTIAL REPAY
        console.log("\n=== TEST 4: PARTIAL REPAY ===");
        uint256 daiBeforeRepay = IERC20(DAI_ADDR).balanceOf(address(account));
        uint256 debtBeforeRepay = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));

        vm.prank(processor);
        aaveManager.repay(repayAmount);
        console.log("After repaying %s DAI:", repayAmount / 10 ** 18);
        logBalances();

        // Verify that DAI balance decreased by the repaid amount
        uint256 daiAfterRepay = IERC20(DAI_ADDR).balanceOf(address(account));
        require(daiAfterRepay <= daiBeforeRepay - repayAmount + 10, "DAI balance didn't decrease correctly after repay");

        // Verify that debt token decreased
        uint256 debtAfterRepay = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));
        require(debtAfterRepay <= debtBeforeRepay - repayAmount + 10, "Debt not decreased correctly after repay");

        // TEST 5: SUPPLY DAI (to get aDAI for repayWithATokens test)
        console.log("\n=== TEST 5: SUPPLY DAI ===");
        // Instead of changing the config to supply DAI, we are going to execute it directly from the account by the owner
        uint256 daiSupplyAmount = 20_000 * 10 ** 18; // 20,000 DAI
        uint256 daiBeforeSupplyDai = IERC20(DAI_ADDR).balanceOf(address(account));
        uint256 aDaiBeforeSupply = IERC20(ADAI_ADDR).balanceOf(address(account));

        vm.startPrank(owner);
        // First approve DAI spending by Aave pool
        bytes memory approveDAICall = abi.encodeCall(IERC20.approve, (AAVE_POOL_ADDR, daiSupplyAmount));
        account.execute(DAI_ADDR, 0, approveDAICall);

        // Now supply DAI to get aDAI tokens
        bytes memory encodedSupplyCall = abi.encodeCall(IPool.supply, (DAI_ADDR, daiSupplyAmount, address(account), 0));
        account.execute(AAVE_POOL_ADDR, 0, encodedSupplyCall);
        vm.stopPrank();

        console.log("After supplying 20,000 DAI to get aDAI:");
        logBalances();

        // Verify that DAI balance decreased
        uint256 daiAfterSupplyDai = IERC20(DAI_ADDR).balanceOf(address(account));
        require(
            daiAfterSupplyDai <= daiBeforeSupplyDai - daiSupplyAmount + 10,
            "DAI balance didn't decrease correctly after supplying DAI"
        );

        // Verify that aDAI balance increased
        uint256 aDaiAfterSupply = IERC20(ADAI_ADDR).balanceOf(address(account));
        require(aDaiAfterSupply > aDaiBeforeSupply, "aDAI balance didn't increase after supplying DAI");

        // TEST 6: REPAY WITH ATOKENS
        console.log("\n=== TEST 6: REPAY WITH ATOKENS ===");
        // Now try repayWithATokens
        uint256 repayWithATokensAmount = 3_000 * 10 ** 18; // 3,000 DAI equivalent in aDAI
        uint256 aDaiBeforeRepay = IERC20(ADAI_ADDR).balanceOf(address(account));
        uint256 debtBeforeATokenRepay = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));

        vm.prank(processor);
        aaveManager.repayWithATokens(repayWithATokensAmount);
        console.log("After repaying %s DAI with aDAI tokens:", repayWithATokensAmount / 10 ** 18);
        logBalances();

        // Verify that aDAI balance decreased
        uint256 aDaiAfterRepay = IERC20(ADAI_ADDR).balanceOf(address(account));
        require(aDaiAfterRepay < aDaiBeforeRepay, "aDAI balance didn't decrease after repaying with aTokens");

        // Verify that debt decreased
        uint256 debtAfterATokenRepay = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));
        require(debtAfterATokenRepay < debtBeforeATokenRepay, "Debt didn't decrease after repaying with aTokens");
        require(
            debtBeforeATokenRepay - debtAfterATokenRepay >= repayWithATokensAmount - 10,
            "Debt didn't decrease by the expected amount after repaying with aTokens"
        );

        // TEST 7: REPAY ALL
        console.log("\n=== TEST 7: REPAY ALL ===");

        vm.prank(processor);
        aaveManager.repay(0); // 0 means repay all
        console.log("After repaying all remaining DAI:");
        logBalances();

        // Verify that debt is fully repaid (or close to zero)
        uint256 debtAfterRepayAll = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));
        require(debtAfterRepayAll == 0, "Debt not fully repaid");

        // TEST 8: WITHDRAW ALL
        console.log("\n=== TEST 8: WITHDRAW ALL ===");
        uint256 usdcBeforeWithdrawAll = IERC20(USDC_ADDR).balanceOf(address(account));

        vm.prank(processor);
        aaveManager.withdraw(type(uint256).max); // max means withdraw all
        console.log("After withdrawing all USDC:");
        logBalances();

        // Verify that aUSDC is fully withdrawn
        uint256 aUsdcAfterWithdrawAll = IERC20(AUSDC_ADDR).balanceOf(address(account));
        require(aUsdcAfterWithdrawAll == 0, "aUSDC not fully withdrawn");

        // Verify that USDC balance increased
        uint256 usdcAfterWithdrawAll = IERC20(USDC_ADDR).balanceOf(address(account));
        require(usdcAfterWithdrawAll > usdcBeforeWithdrawAll, "USDC balance didn't increase after withdrawing all");

        // Final verification
        uint256 finalAUsdcBalance = IERC20(AUSDC_ADDR).balanceOf(address(account));
        uint256 finalDebtBalance = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));

        console.log("\n=== FINAL VERIFICATION ===");

        if (finalAUsdcBalance < 100) {
            // Allow for some dust
            console.log("Supply and withdrawal tests passed successfully!");
        } else {
            console.log("Not all aUSDC withdrawn. Remaining: %s", finalAUsdcBalance / 10 ** 6);
            revert("Supply and withdrawal tests failed: Not all aUSDC withdrawn");
        }

        if (finalDebtBalance < 100) {
            // Allow for some dust
            console.log("Borrow and repay tests passed successfully!");
        } else {
            console.log("Not all DAI debt repaid. Remaining: %s", finalDebtBalance / 10 ** 18);
            revert("Borrow and repay tests failed: Not all DAI debt repaid");
        }

        console.log("\nAavePositionManager integration tests completed successfully!");
    }

    function logBalances() internal view {
        // Log USDC balances
        uint256 usdcBalance = IERC20(USDC_ADDR).balanceOf(address(account));
        uint256 aUsdcBalance = IERC20(AUSDC_ADDR).balanceOf(address(account));
        console.log("USDC balance: %s", usdcBalance / 10 ** 6);
        console.log("aUSDC balance: %s", aUsdcBalance / 10 ** 6);

        // Log DAI balances
        uint256 daiBalance = IERC20(DAI_ADDR).balanceOf(address(account));
        uint256 aDaiBalance = IERC20(ADAI_ADDR).balanceOf(address(account));
        uint256 debtDaiBalance = IERC20(VARIABLE_DEBT_DAI).balanceOf(address(account));
        console.log("DAI balance: %s", daiBalance / 10 ** 18);
        console.log("aDAI balance: %s", aDaiBalance / 10 ** 18);
        console.log("DAI debt: %s", debtDaiBalance / 10 ** 18);

        // Get health factor if possible
        try IPool(AAVE_POOL_ADDR).getUserAccountData(address(account)) returns (
            uint256 totalCollateralBase,
            uint256 totalDebtBase,
            uint256 availableBorrowsBase,
            uint256, // currentLiquidationThreshold
            uint256, // ltv
            uint256 healthFactor
        ) {
            console.log("Total collateral (USD): %s", totalCollateralBase / 10 ** 8);
            console.log("Total debt (USD): %s", totalDebtBase / 10 ** 8);
            console.log("Available borrow (USD): %s", availableBorrowsBase / 10 ** 8);
            console.log("Health factor: %s", healthFactor / 10 ** 18);
        } catch {
            console.log("Could not fetch position metrics");
        }
    }
}
