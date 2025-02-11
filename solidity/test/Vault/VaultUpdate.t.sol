// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {VaultHelper} from "./VaultHelper.t.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {console} from "forge-std/src/console.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";

contract VaultUpdateTest is VaultHelper {
    using Math for uint256;

    function testUpdateRevertsWithZeroRate() public {
        vm.startPrank(strategist);
        vm.expectRevert(ValenceVault.InvalidRate.selector);
        vault.update(0, 0, 0);
        vm.stopPrank();
    }

    function testUpdateRevertsWithHighWithdrawFee() public {
        vm.startPrank(strategist);
        vm.expectRevert(ValenceVault.InvalidWithdrawFee.selector);
        vault.update(ONE_SHARE, MAX_WITHDRAW_FEE + 1, 0);
        vm.stopPrank();
    }

    function testUpdateRevertsWhenNotStrategist() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.OnlyStrategistAllowed.selector);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();
    }

    function testUpdateSuccessWithNoFees() public {
        // Setup initial deposit
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        uint256 newRate = ONE_SHARE + 100; // 1.01x
        uint32 newWithdrawFee = 100; // 1%

        vm.startPrank(strategist);
        vault.update(newRate, newWithdrawFee, 0);
        vm.stopPrank();

        assertEq(vault.redemptionRate(), newRate);
        assertEq(vault.feesOwedInAsset(), 0);
    }

    function testUpdateCollectsAndDistributesFees() public {
        // Setup fees
        setFees(0, 1000, 2000, 0); // 10% platform fee, 20% performance fee

        // Setup initial deposit and state
        uint256 depositAmount = 100000;
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // First update to initialize LastUpdateTotalShares
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        // Move time forward 6 months
        vm.warp(vm.getBlockTimestamp() + 180 days);

        uint256 newRate = ONE_SHARE.mulDiv(BASIS_POINTS + 1000, BASIS_POINTS); // 1.10x increase

        vm.startPrank(strategist);
        vault.update(newRate, 0, 0);
        vm.stopPrank();

        // Calculate expected platform fees (half year)
        uint256 platformFees = depositAmount.mulDiv(1000, BASIS_POINTS).mulDiv(180, 365);

        // Calculate expected performance fees
        uint256 yield = depositAmount.mulDiv(1000, BASIS_POINTS); // 10% increase
        uint256 performanceFees = yield.mulDiv(2000, BASIS_POINTS); // 20% of yield

        // Total fees and expected distribution
        uint256 totalFees = platformFees + performanceFees;
        uint256 expectedStrategistShares = totalFees.mulDiv(3000, BASIS_POINTS);
        uint256 expectedPlatformShares = totalFees - expectedStrategistShares;

        assertEq(vault.balanceOf(strategistFeeAccount), expectedStrategistShares);
        assertEq(vault.balanceOf(platformFeeAccount), expectedPlatformShares);
        assertEq(vault.feesOwedInAsset(), 0);
    }

    function testFeesDistributionRatio() public {
        // Setup fees and initial deposit
        setFees(0, 1000, 0, 0); // 10% platform fee only

        vm.startPrank(user);
        uint256 depositAmount = 100000;
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // First update to initialize LastUpdateTotalShares
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        // Move time forward 1 year for easy fee calculation
        vm.warp(vm.getBlockTimestamp() + 365 days);

        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        // Calculate expected fees
        uint256 totalFees = depositAmount.mulDiv(1000, BASIS_POINTS); // 10000
        uint256 expectedStrategistShares = totalFees.mulDiv(3000, BASIS_POINTS); // 3000
        uint256 expectedPlatformShares = totalFees - expectedStrategistShares; // 7000

        assertEq(vault.balanceOf(strategistFeeAccount), expectedStrategistShares);
        assertEq(vault.balanceOf(platformFeeAccount), expectedPlatformShares);
    }

    function testUpdateDistributesDepositFees() public {
        // Setup deposit fee
        setFees(500, 0, 0, 0); // 5% deposit fee only

        // Make deposit to collect fees
        vm.startPrank(user);
        uint256 depositAmount = 100000;
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // Update should distribute collected deposit fees
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        // Calculate expected fees
        uint256 totalFees = depositAmount.mulDiv(500, BASIS_POINTS); // 5000
        uint256 expectedStrategistShares = totalFees.mulDiv(3000, BASIS_POINTS); // 1500
        uint256 expectedPlatformShares = totalFees - expectedStrategistShares; // 3500

        assertEq(vault.balanceOf(strategistFeeAccount), expectedStrategistShares);
        assertEq(vault.balanceOf(platformFeeAccount), expectedPlatformShares);
    }

    function testUpdateDistributesMultipleFeeTypes() public {
        // Setup multiple fee types
        setFees(500, 1000, 2000, 0); // 5% deposit, 10% platform, 20% performance fee

        // Make deposit
        uint256 depositAmount = 100000;
        vm.startPrank(user);
        vault.deposit(depositAmount, user);
        vm.stopPrank();

        // First update to initialize LastUpdateTotalShares and distribute deposit fees
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        uint256 depositFees = depositAmount.mulDiv(500, BASIS_POINTS); // 5000
        uint256 remainingAssets = depositAmount - depositFees; // 95000

        // Move time forward 1 year
        vm.warp(vm.getBlockTimestamp() + 365 days);

        // Update with 10% profit
        vm.startPrank(strategist);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS + 1000, BASIS_POINTS), 0, 0); // 110% of initial rate
        vm.stopPrank();

        // Calculate platform fees on remaining assets
        uint256 platformFees = remainingAssets.mulDiv(1000, BASIS_POINTS); // 9500

        // Calculate performance fees on the yield
        uint256 yield = depositAmount.mulDiv(1000, BASIS_POINTS); // 9500
        uint256 performanceFees = yield.mulDiv(2000, BASIS_POINTS); // 1900

        uint256 totalFees = depositFees + platformFees + performanceFees; // 16400
        uint256 expectedStrategistShares = totalFees.mulDiv(3000, BASIS_POINTS); // 4920
        uint256 expectedPlatformShares = totalFees - expectedStrategistShares; // 11480

        assertEq(vault.balanceOf(strategistFeeAccount), expectedStrategistShares, "Strategist shares mismatch");
        assertEq(vault.balanceOf(platformFeeAccount), expectedPlatformShares, "Platform shares mismatch");
        assertEq(vault.feesOwedInAsset(), 0, "Fees owed should be 0");
    }

    function testHandleWithdrawNetting() public {
        // Setup initial deposit
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        uint256 nettingAmount = 50000;

        // Make sure we get the correct call to execute
        vm.expectCall(
            address(token),
            abi.encodeWithSignature("transfer(address,uint256)", address(withdrawAccount), nettingAmount)
        );

        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, nettingAmount);
        vm.stopPrank();
    }

    function testUpdateIncrementId() public {
        // Setup initial state
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        ValenceVault.PackedValues memory packedValues = _getPackedValues();
        uint64 initialUpdateId = packedValues.currentUpdateId;

        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, 0);
        vm.stopPrank();

        packedValues = _getPackedValues();

        assertEq(packedValues.currentUpdateId, initialUpdateId + 1);
    }

    function testUpdateInfoStorage() public {
        // Setup initial state
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        uint256 newRate = ONE_SHARE.mulDiv(BASIS_POINTS + 500, BASIS_POINTS); // 1.05x
        uint32 withdrawFee = 100; // 1%
        uint256 expectedWithdrawRate = ONE_SHARE.mulDiv(BASIS_POINTS - withdrawFee, BASIS_POINTS);

        vm.startPrank(strategist);
        vault.update(newRate, withdrawFee, 0);
        vm.stopPrank();

        ValenceVault.PackedValues memory packedValues = _getPackedValues();

        (uint256 storedRate, uint64 storedTimestamp, uint32 _withdrawFee) =
            vault.updateInfos(packedValues.currentUpdateId);
        assertEq(storedRate, expectedWithdrawRate);
        assertEq(withdrawFee, _withdrawFee);
        assertEq(storedTimestamp, vm.getBlockTimestamp());
    }

    function testMaxHistoricalRateUpdate() public {
        // Setup initial state
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        uint256 higherRate = ONE_SHARE.mulDiv(BASIS_POINTS + 500, BASIS_POINTS);
        uint256 lowerRate = ONE_SHARE.mulDiv(BASIS_POINTS - 500, BASIS_POINTS);

        // First update with higher rate
        vm.startPrank(strategist);
        vault.update(higherRate, 0, 0);
        assertEq(vault.maxHistoricalRate(), higherRate);

        // Update with lower rate shouldn't change maxHistoricalRate
        _update(lowerRate, 0, 0);
        assertEq(vault.maxHistoricalRate(), higherRate);
        vm.stopPrank();
    }

    function testUpdateRevertsSameBlock() public {
        // Setup initial state
        vm.startPrank(user);
        vault.deposit(100000, user);
        vm.stopPrank();

        // Switch to strategist
        vm.startPrank(strategist);

        // First update should succeed
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS + 100, BASIS_POINTS), 0, 0); // 1.01x rate

        // Second update in same block should fail
        vm.expectRevert(ValenceVault.InvalidUpdateSameBlock.selector);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS + 200, BASIS_POINTS), 0, 0); // 1.02x rate

        vm.stopPrank();
    }

    function testUpdateSucceedsNextBlock() public {
        // Setup initial state
        vm.startPrank(user);
        vault.deposit(1000, user);
        vm.stopPrank();

        // Switch to strategist
        vm.startPrank(strategist);

        // First update
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS + 100, BASIS_POINTS), 0, 0); // 1.01x rate

        // Move to next block
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12); // Assuming ~12 sec block time

        // Second update should now succeed
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS + 200, BASIS_POINTS), 0, 0); // 1.02x rate

        vm.stopPrank();

        // Verify the last update took effect
        assertEq(vault.redemptionRate(), ONE_SHARE.mulDiv(BASIS_POINTS + 200, BASIS_POINTS));
    }
}
