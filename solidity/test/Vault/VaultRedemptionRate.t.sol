// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";

contract VaultRedemptionTest is VaultHelper {
    using Math for uint256;

    // Test single deposit redemption rate calculations
    function testBasicRedemptionRateCalculation() public {
        uint256 depositAmount = 100_000;

        // Initial deposit at 1:1 rate
        vm.startPrank(user);
        uint256 shares = vault.deposit(depositAmount, user);
        vm.stopPrank();

        // Verify initial shares calculation
        assertEq(shares, depositAmount, "Initial shares mismatch");
        assertEq(vault.balanceOf(user), shares, "User balance mismatch");
        assertEq(vault.totalSupply(), shares, "Total supply mismatch");

        // Update rate to 110% (10% profit)
        uint256 newRate = ONE_SHARE.mulDiv(BASIS_POINTS + 1000, BASIS_POINTS);
        vm.startPrank(strategist);
        _update(newRate, 0, 0);
        vm.stopPrank();

        // Verify asset conversion
        uint256 expectedAssets = shares.mulDiv(newRate, ONE_SHARE);
        assertEq(vault.convertToAssets(shares), expectedAssets, "Asset conversion mismatch");
        assertEq(vault.totalAssets(), expectedAssets, "Total assets mismatch");
    }

    // Test multiple deposits with rate changes
    function testMultipleDepositsWithRateChanges() public {
        // First deposit at 1:1
        vm.startPrank(user);
        uint256 firstShares = vault.deposit(100_000, user);
        vm.stopPrank();

        // Update rate to 120% (20% profit)
        uint256 firstNewRate = ONE_SHARE.mulDiv(BASIS_POINTS + 2000, BASIS_POINTS);
        vm.startPrank(strategist);
        _update(firstNewRate, 0, 0);
        vm.stopPrank();

        // Second deposit at new rate
        address user2 = makeAddr("user2");
        vm.startPrank(owner);
        token.mint(user2, INITIAL_USER_BALANCE);
        vm.stopPrank();

        vm.startPrank(user2);
        token.approve(address(vault), type(uint256).max);
        uint256 secondDeposit = 50_000;
        uint256 secondShares = vault.deposit(secondDeposit, user2);
        vm.stopPrank();

        // Verify second deposit shares
        uint256 expectedSecondShares = secondDeposit.mulDiv(ONE_SHARE, firstNewRate);
        assertEq(secondShares, expectedSecondShares, "Second deposit shares mismatch");

        // Update rate to 150% (25% additional profit)
        uint256 secondNewRate = ONE_SHARE.mulDiv(150, 100);
        vm.startPrank(strategist);
        _update(secondNewRate, 0, 0);
        vm.stopPrank();

        // Verify final assets for both users
        uint256 user1Assets = vault.convertToAssets(firstShares);
        uint256 user2Assets = vault.convertToAssets(secondShares);
        uint256 expectedUser1Assets = firstShares.mulDiv(secondNewRate, ONE_SHARE);
        uint256 expectedUser2Assets = secondShares.mulDiv(secondNewRate, ONE_SHARE);

        assertEq(user1Assets, expectedUser1Assets, "User1 final assets mismatch");
        assertEq(user2Assets, expectedUser2Assets, "User2 final assets mismatch");
    }

    // Test withdrawal calculations with fees
    function testWithdrawalCalculationsWithFees() public {
        uint256 depositAmount = 100_000;
        uint32 withdrawFeeBps = 100; // 1% withdraw fee

        // Initial deposit
        vm.startPrank(user);
        uint256 shares = vault.deposit(depositAmount, user);
        vm.stopPrank();

        uint256 userPreBalance = token.balanceOf(user);

        // Update rate
        vm.startPrank(strategist);
        _update(ONE_SHARE, withdrawFeeBps, shares);
        vm.stopPrank();

        // Calculate expected withdraw rate (rate after fee)
        uint256 expectedWithdrawRate = ONE_SHARE.mulDiv(BASIS_POINTS - withdrawFeeBps, BASIS_POINTS);

        // Request withdrawal
        vm.startPrank(user);
        vault.redeem(shares / 10, user, user, 500, false); // Accept up to 5% loss
        vm.stopPrank();

        vm.startPrank(strategist);
        _update(ONE_SHARE, withdrawFeeBps, 0);
        vm.stopPrank();

        // Verify withdrawal request details
        (address owner, uint64 claimTime, uint32 maxLossBps, address receiver,,, uint256 sharesAmount) =
            vault.userWithdrawRequest(user);

        // Convert shares to uint128 for comparison
        uint128 expectedSharesAmount = uint128(shares / 10);
        assertEq(sharesAmount, expectedSharesAmount, "Withdraw shares mismatch");
        assertEq(maxLossBps, 500, "Max loss mismatch");
        assertEq(owner, user, "Owner mismatch");
        assertEq(receiver, user, "Receiver mismatch");

        // Fast forward past lockup
        vm.warp(claimTime + 1);

        // Complete withdrawal
        vm.startPrank(user);
        vault.completeWithdraw(user);
        vm.stopPrank();

        // Verify final token balance using the actual sharesAmount from the request
        uint256 expectedAssets = uint256(sharesAmount).mulDiv(expectedWithdrawRate, ONE_SHARE);
        assertEq(token.balanceOf(user), userPreBalance + expectedAssets, "Final withdrawn assets mismatch");
    }

    // Test complex scenario with multiple operations
    function testComplexRedemptionScenario() public {
        // Setup multiple users
        address[] memory users = new address[](3);
        users[0] = user;
        users[1] = makeAddr("user2");
        users[2] = makeAddr("user3");

        // Mint tokens for all users
        for (uint256 i = 1; i < users.length; i++) {
            vm.startPrank(owner);
            token.mint(users[i], INITIAL_USER_BALANCE);
            vm.stopPrank();

            vm.startPrank(users[i]);
            token.approve(address(vault), type(uint256).max);
            vm.stopPrank();
        }
        token.mint(address(_getConfig().withdrawAccount), INITIAL_USER_BALANCE * 2);

        // Initial deposits at 1:1
        uint256[] memory deposits = new uint256[](3);
        deposits[0] = 100_000;
        deposits[1] = 50_000;
        deposits[2] = 75_000;

        uint256[] memory shares = new uint256[](3);
        for (uint256 i = 0; i < users.length; i++) {
            vm.startPrank(users[i]);
            shares[i] = vault.deposit(deposits[i], users[i]);
            vm.stopPrank();
        }

        // First rate update
        vm.startPrank(strategist);
        _update(ONE_SHARE, 100, 0); // 1% withdraw fee
        vm.stopPrank();

        // User 1 withdraws half
        vm.startPrank(users[0]);
        vault.redeem(shares[0] / 2, users[0], users[0], 200, false);
        vm.stopPrank();

        // Second rate update (10% total profit)
        uint256 rate110 = ONE_SHARE.mulDiv(BASIS_POINTS + 1000, BASIS_POINTS);
        vm.startPrank(strategist);
        _update(rate110, 100, 0);
        vm.stopPrank();

        // User 2 deposits more
        vm.startPrank(users[1]);
        uint256 additionalShares = vault.deposit(25_000, users[1]);
        shares[1] += additionalShares;
        vm.stopPrank();

        // Fast forward and complete User 1's withdrawal
        vm.warp(vm.getBlockTimestamp() + 4 days);
        vm.startPrank(users[0]);
        vault.completeWithdraw(users[0]);
        vm.stopPrank();

        // User 2 withdraws 1/3
        vm.startPrank(users[1]);
        vault.redeem(shares[1] / 3, users[1], users[1], 200, false);
        vm.stopPrank();

        // Final rate update (20% total profit)
        uint256 rate120 = ONE_SHARE.mulDiv(BASIS_POINTS + 2000, BASIS_POINTS);
        vm.startPrank(strategist);
        _update(rate120, 100, 0);
        vm.stopPrank();

        uint256 user2PreWithdrawBalance = token.balanceOf(users[1]);

        vm.warp(vm.getBlockTimestamp() + 4 days);
        vm.startPrank(users[1]);
        vault.completeWithdraw(users[1]);
        vm.stopPrank();

        // Final state verification
        {
            // User 1 final state
            uint256 user1RemainingShares = vault.balanceOf(users[0]);
            uint256 user1RemainingAssets = vault.convertToAssets(user1RemainingShares);
            assertEq(user1RemainingShares, shares[0] / 2, "User1 remaining shares incorrect");
            assertEq(user1RemainingAssets, 60_000, "User1 remaining assets incorrect"); // 50,000 * 1.2

            // User 2 final state
            uint256 user2RemainingShares = vault.balanceOf(users[1]);
            uint256 user2ExpectedShares = shares[1] - (shares[1] / 3); // 2/3 of total shares remaining
            assertEq(user2RemainingShares, user2ExpectedShares, "User2 remaining shares incorrect");
            // Their withdrawal at 110% with 1% fee: (shares[1]/3) * 1.1 * 0.99
            uint256 user2WithdrawnAmount =
                (shares[1] / 3).mulDiv(rate110, ONE_SHARE).mulDiv(BASIS_POINTS - 100, BASIS_POINTS);
            assertApproxEqRel(
                token.balanceOf(users[1]),
                user2PreWithdrawBalance + user2WithdrawnAmount,
                1e15,
                "User2 withdrawn amount incorrect"
            );
            // Remaining assets at 120%
            uint256 user2RemainingAssets = vault.convertToAssets(user2RemainingShares);
            uint256 user2ExpectedAssets = user2RemainingShares.mulDiv(rate120, ONE_SHARE, Math.Rounding.Floor);
            assertApproxEqRel(user2RemainingAssets, user2ExpectedAssets, 1e15, "User2 remaining assets incorrect");

            // User 3 final state (never withdrew)
            uint256 user3RemainingShares = vault.balanceOf(users[2]);
            uint256 user3RemainingAssets = vault.convertToAssets(user3RemainingShares);
            assertEq(user3RemainingShares, shares[2], "User3 shares should be unchanged");
            assertEq(user3RemainingAssets, 90_000, "User3 final assets incorrect"); // 75,000 * 1.2

            // Global state
            uint256 expectedTotalShares = shares[0] / 2 + shares[1] - (shares[1] / 3) + shares[2];
            assertEq(vault.totalSupply(), expectedTotalShares, "Total supply incorrect");
            uint256 expectedTotalAssets = vault.convertToAssets(expectedTotalShares);
            assertEq(vault.totalAssets(), expectedTotalAssets, "Total assets incorrect");
        }
    }
}
