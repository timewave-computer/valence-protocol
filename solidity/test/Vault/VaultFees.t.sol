// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";
import {console} from "forge-std/src/console.sol";

contract ValenceVaultFeeTest is VaultHelper {
    using Math for uint256;

    uint32 constant DEPOSIT_FEE_BPS = 500; // 5%
    uint32 constant PLATFORM_FEE_BPS = 1000; // 10%
    uint32 constant PERFORMANCE_FEE_BPS = 2000; // 20%

    function testDepositFeeCalculation() public {
        setFees(DEPOSIT_FEE_BPS, 0, 0, 0);
        uint256 depositAmount = 10_000;

        // Test deposit fee calculation
        uint256 expectedFee = (depositAmount * DEPOSIT_FEE_BPS) / BASIS_POINTS;
        uint256 calculatedFee = vault.calculateDepositFee(depositAmount);
        assertEq(calculatedFee, expectedFee, "Deposit fee calculation mismatch");

        // Test mint fee calculation
        uint256 sharesToMint = 9_500; // Should require 10_000 input for 5% fee
        (uint256 grossAssets, uint256 fee) = vault.calculateMintFee(sharesToMint);

        assertEq(fee, expectedFee, "Mint fee calculation mismatch");
        assertEq(grossAssets, depositAmount, "Gross assets calculation mismatch");
    }

    function testDepositWithFee() public {
        setFees(DEPOSIT_FEE_BPS, 0, 0, 0);
        vm.startPrank(user);

        uint256 depositAmount = 10_000;
        uint256 expectedFee = (depositAmount * DEPOSIT_FEE_BPS) / BASIS_POINTS;
        uint256 expectedShares = depositAmount - expectedFee;

        vault.deposit(depositAmount, user);

        assertEq(vault.balanceOf(user), expectedShares, "User should receive shares minus fee");
        assertEq(vault.feesOwedInAsset(), expectedFee, "Fee not collected correctly");
        vm.stopPrank();
    }

    function testMintWithFee() public {
        setFees(DEPOSIT_FEE_BPS, 0, 0, 0);
        vm.startPrank(user);

        uint256 sharesToMint = 9_500;
        (uint256 requiredAssets, uint256 expectedFee) = vault.calculateMintFee(sharesToMint);

        uint256 preBalance = token.balanceOf(user);
        vault.mint(sharesToMint, user);

        assertEq(vault.balanceOf(user), sharesToMint, "User should receive requested shares");
        assertEq(token.balanceOf(user), preBalance - requiredAssets, "Incorrect assets taken");
        assertEq(vault.feesOwedInAsset(), expectedFee, "Fee not collected correctly");
        vm.stopPrank();
    }

    function testPlatformFee() public {
        // Setup
        setFees(0, PLATFORM_FEE_BPS, 0, 0);
        uint256 initialDeposit = 10_000;
        uint256 period = 91.25 days;

        // Initial deposit
        vm.prank(user);
        vault.deposit(initialDeposit, user);

        vm.startPrank(strategist);

        // First period
        vm.warp(vm.getBlockTimestamp() + period);

        uint256 initialFeesOwed = vault.feesOwedInAsset();
        vault.update(BASIS_POINTS, 0, 0);
        uint256 firstPeriodFees =
            vault.balanceOf(platformFeeAccount) + vault.balanceOf(strategistFeeAccount) - initialFeesOwed;

        vm.warp(vm.getBlockTimestamp() + period);

        vault.update(BASIS_POINTS, 0, 0);
        uint256 secondPeriodFees =
            vault.balanceOf(platformFeeAccount) + vault.balanceOf(strategistFeeAccount) - firstPeriodFees;

        vm.stopPrank();

        // Calculate expected fee
        uint256 expectedPeriodFee = initialDeposit.mulDiv(PLATFORM_FEE_BPS, BASIS_POINTS).mulDiv(period, 365 days);

        // Assert first period has no fees (LastUpdateTotalShares was 0)
        assertEq(firstPeriodFees, 0, "First period should have no fees");
        // Assert second period has expected fees
        assertEq(secondPeriodFees, expectedPeriodFee, "Second period fees incorrect");
    }

    function testPerformanceFee() public {
        setFees(0, 0, PERFORMANCE_FEE_BPS, 0);

        uint256 depositAmount = 10_000;
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        vm.startPrank(strategist);
        // Update with 50% increase
        uint32 newRate = (BASIS_POINTS * 15) / 10; // 1.5x
        uint256 initialFeesOwed = vault.feesOwedInAsset();
        vault.update(newRate, 0, 0);

        // Calculate yield and fee
        uint256 totalYield = depositAmount.mulDiv(newRate - BASIS_POINTS, BASIS_POINTS, Math.Rounding.Floor);
        uint256 expectedFee = totalYield.mulDiv(PERFORMANCE_FEE_BPS, BASIS_POINTS, Math.Rounding.Floor);
        uint256 actualFee =
            vault.balanceOf(platformFeeAccount) + vault.balanceOf(strategistFeeAccount) - initialFeesOwed;

        assertEq(actualFee, expectedFee, "Performance fee calculation incorrect");
        assertEq(vault.maxHistoricalRate(), newRate, "Max historical rate not updated");
        vm.stopPrank();
    }

    function testNoPerformanceFeeBelowHighWater() public {
        setFees(0, 0, PERFORMANCE_FEE_BPS, 0);

        vm.startPrank(user);
        vault.deposit(10_000, user);
        vm.stopPrank();

        vm.startPrank(strategist);
        // First update with 50% increase
        uint32 highRate = (BASIS_POINTS * 15) / 10; // 1.5x
        vault.update(highRate, 0, 0);
        uint256 feesAfterIncrease = vault.feesOwedInAsset();

        // Second update with lower rate
        uint32 lowerRate = (BASIS_POINTS * 13) / 10; // 1.3x
        _update(lowerRate, 0, 0);

        assertEq(vault.feesOwedInAsset(), feesAfterIncrease, "No new fees should be collected below high water");
        assertEq(vault.maxHistoricalRate(), highRate, "High water mark should not change");
        vm.stopPrank();
    }

    function testCombinedFees() public {
        setFees(DEPOSIT_FEE_BPS, PLATFORM_FEE_BPS, PERFORMANCE_FEE_BPS, 0);

        uint256 depositAmount = 10_000;

        // Test deposit fee
        vm.startPrank(user);
        uint256 depositFee = (depositAmount * DEPOSIT_FEE_BPS) / BASIS_POINTS;

        vault.deposit(depositAmount, user);
        vm.stopPrank();

        assertEq(vault.feesOwedInAsset(), depositFee, "Initial deposit fee incorrect");

        // Initial update to set LastUpdateTotalShares
        vm.startPrank(strategist);
        vault.update(BASIS_POINTS, 0, 0); // Update with 1:1 rate
        vm.stopPrank();

        // Skip 6 months and update with 50% increase
        vm.warp(vm.getBlockTimestamp() + 182.5 days);

        vm.startPrank(strategist);
        uint32 newRate = (BASIS_POINTS * 15) / 10; // 1.5x

        uint256 preUpdateFees = vault.feesOwedInAsset();

        // Calculate platform fee
        uint256 assetsForPlatformFee = depositAmount - depositFee;

        uint256 expectedPlatformFee = assetsForPlatformFee.mulDiv(PLATFORM_FEE_BPS, BASIS_POINTS, Math.Rounding.Floor)
            .mulDiv(182.5 days, 365 days, Math.Rounding.Floor);

        // Calculate performance fee
        uint256 totalYield = depositAmount.mulDiv(newRate - BASIS_POINTS, BASIS_POINTS, Math.Rounding.Floor);

        uint256 expectedPerformanceFee = totalYield.mulDiv(PERFORMANCE_FEE_BPS, BASIS_POINTS, Math.Rounding.Floor);

        vault.update(newRate, 0, 0);

        // Final fee checks
        uint256 totalNewFees =
            vault.balanceOf(platformFeeAccount) + vault.balanceOf(strategistFeeAccount) - preUpdateFees;

        assertEq(
            totalNewFees,
            depositFee + expectedPlatformFee + expectedPerformanceFee,
            "Combined fee calculation incorrect"
        );
        vm.stopPrank();
    }

    function testNoFeeAccumulationAfterUpdates() public {
        // Setup multiple fee types
        setFees(500, 1000, 2000, 0); // 5% deposit, 10% platform, 20% performance fee

        // Make initial deposit to generate deposit fees
        uint256 depositAmount = 100000;
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // First update - should distribute deposit fees
        vm.startPrank(strategist);
        _update(BASIS_POINTS, 0, 0);
        assertEq(vault.feesOwedInAsset(), 0, "Fees should be zero after first update");

        // Move time forward and update with profit to generate platform and performance fees
        _update(BASIS_POINTS + 500, 0, 0); // 5% profit
        assertEq(vault.feesOwedInAsset(), 0, "Fees should be zero after second update");

        // Another time period and rate change
        _update(BASIS_POINTS + 1000, 0, 0); // 10% profit
        assertEq(vault.feesOwedInAsset(), 0, "Fees should be zero after third update");

        // Make another deposit to generate more deposit fees
        vm.stopPrank();
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // Final update to distribute new deposit fees
        vm.startPrank(strategist);
        _update(BASIS_POINTS + 1500, 0, 0); // 15% profit
        assertEq(vault.feesOwedInAsset(), 0, "Fees should be zero after fourth update");
        vm.stopPrank();
    }

    function testFeeDistribution() public {
        // Setup fee distribution ratio (30% to strategist, 70% to platform)
        uint32 strategistRatio = 3000; // 30% in basis points
        setFeeDistribution(strategistFeeAccount, platformFeeAccount, strategistRatio);

        // Set deposit fee only for simplicity (5%)
        setFees(500, 0, 0, 0);

        // Calculate expected fee splits for a 10,000 token deposit
        uint256 depositAmount = 10_000;
        uint256 expectedTotalFee = (depositAmount * 500) / BASIS_POINTS; // 500 tokens

        // Expected splits:
        // Strategist (30%): 150 tokens worth of shares
        // Platform (70%): 350 tokens worth of shares
        uint256 expectedStrategistFee = (expectedTotalFee * strategistRatio) / BASIS_POINTS; // 150
        uint256 expectedPlatformFee = expectedTotalFee - expectedStrategistFee; // 350

        // Initial balances should be 0
        assertEq(vault.balanceOf(strategistFeeAccount), 0, "Initial strategist balance should be 0");
        assertEq(vault.balanceOf(platformFeeAccount), 0, "Initial platform balance should be 0");

        // Make deposit to generate fees
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // Update to trigger fee distribution
        vm.prank(strategist);
        _update(BASIS_POINTS, 0, 0);

        // Verify fee distribution
        uint256 strategistShares = vault.balanceOf(strategistFeeAccount);
        uint256 platformShares = vault.balanceOf(platformFeeAccount);

        assertEq(strategistShares, expectedStrategistFee, "Incorrect strategist fee distribution");
        assertEq(platformShares, expectedPlatformFee, "Incorrect platform fee distribution");

        // Total distributed fees should equal expected total fee
        assertEq(strategistShares + platformShares, expectedTotalFee, "Total distributed fees mismatch");

        // feesOwedInAsset should be 0 after distribution
        assertEq(vault.feesOwedInAsset(), 0, "Fees owed should be 0 after distribution");
    }

    function testFeeDistributionWithMultipleUpdates() public {
        // Setup fee distribution (30% to strategist, 70% to platform)
        uint32 strategistRatio = 3000;
        setFeeDistribution(strategistFeeAccount, platformFeeAccount, strategistRatio);

        // Set both deposit (5%) and performance (20%) fees
        setFees(500, 0, 2000, 0);

        uint256 depositAmount = 10_000;

        // First deposit and update - only deposit fees
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        vm.startPrank(strategist);
        _update(BASIS_POINTS, 0, 0);

        // Calculate and verify first distribution (deposit fees)
        uint256 firstFee = (depositAmount * 500) / BASIS_POINTS; // 500 tokens
        uint256 expectedFirstStrategistFee = (firstFee * strategistRatio) / BASIS_POINTS; // 150
        uint256 expectedFirstPlatformFee = firstFee - expectedFirstStrategistFee; // 350

        assertEq(
            vault.balanceOf(strategistFeeAccount),
            expectedFirstStrategistFee,
            "First strategist fee distribution incorrect"
        );
        assertEq(
            vault.balanceOf(platformFeeAccount), expectedFirstPlatformFee, "First platform fee distribution incorrect"
        );

        // Second update with performance increase (50% gain)
        uint32 newRate = (BASIS_POINTS * 15) / 10; // 1.5x
        _update(newRate, 0, 0);

        // Calculate performance fee
        uint256 totalYield = depositAmount.mulDiv(newRate - BASIS_POINTS, BASIS_POINTS);
        uint256 performanceFee = totalYield.mulDiv(2000, BASIS_POINTS); // 20% of yield

        uint256 expectedSecondStrategistFee = (performanceFee * strategistRatio) / BASIS_POINTS;
        uint256 expectedSecondPlatformFee = performanceFee - expectedSecondStrategistFee;

        // Total expected fees after both distributions
        uint256 totalExpectedStrategistFee = expectedFirstStrategistFee + expectedSecondStrategistFee;
        uint256 totalExpectedPlatformFee = expectedFirstPlatformFee + expectedSecondPlatformFee;

        assertEq(
            vault.balanceOf(strategistFeeAccount),
            totalExpectedStrategistFee,
            "Final strategist fee distribution incorrect"
        );
        assertEq(
            vault.balanceOf(platformFeeAccount), totalExpectedPlatformFee, "Final platform fee distribution incorrect"
        );
        vm.stopPrank();
    }
}
