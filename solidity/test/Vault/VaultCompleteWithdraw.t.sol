// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {console} from "forge-std/src/console.sol";
import {VmSafe} from "forge-std/src/Vm.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract VaultCompleteWithdrawTest is VaultHelper {
    using Math for uint256;

    address[] users;
    address solver;
    uint256 constant WITHDRAW_AMOUNT = 1000;
    uint256 constant INITIAL_DEPOSIT_AMOUNT = 10000;
    uint32 constant MAX_LOSS = 500; // 5%
    uint256 constant NUM_USERS = 5;

    event WithdrawCompleted(
        address indexed owner, address indexed receiver, uint256 assets, uint256 shares, address indexed executor
    );
    event WithdrawCompletionSkipped(address indexed owner, string reason);

    function setUp() public override {
        super.setUp();
        solver = makeAddr("solver");
        vm.deal(solver, 1 ether);
        vm.deal(user, 1 ether);

        vm.startPrank(user);
        vault.deposit(INITIAL_DEPOSIT_AMOUNT, user);
        vm.stopPrank();

        // Create multiple users and set them up
        for (uint256 i = 0; i < NUM_USERS; i++) {
            address newUser = makeAddr(string.concat("user", vm.toString(i)));
            users.push(newUser);

            // Setup each user with tokens and approvals
            vm.startPrank(owner);
            token.mint(newUser, INITIAL_USER_BALANCE);
            vm.stopPrank();

            vm.startPrank(newUser);
            token.approve(address(vault), type(uint256).max);
            vault.deposit(INITIAL_DEPOSIT_AMOUNT, newUser);
            vm.stopPrank();

            vm.deal(newUser, 1 ether);
        }

        // Setup initial state for withdraw account
        vm.startPrank(owner);
        token.mint(address(withdrawAccount), INITIAL_USER_BALANCE);
        vm.stopPrank();
    }

    function testCannotCompleteNonExistentWithdraw() public {
        vm.expectRevert(ValenceVault.WithdrawRequestNotFound.selector);
        vault.completeWithdraw(user);
    }

    function testCompleteWithdrawBasicFlow() public {
        uint256 userVaultBalanceBefore = vault.balanceOf(user);
        uint256 userAssetBalanceBefore = token.balanceOf(user);
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        assertEq(vault.balanceOf(user), userVaultBalanceBefore - WITHDRAW_AMOUNT, "User should have reduced shares");

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Get the request info for verification
        (,,,,,, uint256 shares) = vault.userWithdrawRequest(user);

        vm.expectEmit(true, true, true, true);
        emit WithdrawCompleted(user, user, WITHDRAW_AMOUNT, shares, user);

        assertEq(
            token.balanceOf(user), userAssetBalanceBefore, "User should not have more assets before withdraw complete"
        );

        // Complete withdraw
        vm.prank(user);
        vault.completeWithdraw(user);

        // Verify request is cleared
        (,,,,,, uint256 sharesAfter) = vault.userWithdrawRequest(user);
        assertEq(sharesAfter, 0, "Shares should be 0 after completion");

        assertEq(token.balanceOf(user), userAssetBalanceBefore + WITHDRAW_AMOUNT, "User should have increased assets");
    }

    function testCompleteWithdrawWithSolverFee() public {
        setFees(0, 0, 0, 100);

        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw{value: 100}(WITHDRAW_AMOUNT, user, user, MAX_LOSS, true);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Get the request info for verification
        (,,,,,, uint256 shares) = vault.userWithdrawRequest(user);

        vm.expectEmit(true, true, true, true);
        emit WithdrawCompleted(user, user, WITHDRAW_AMOUNT, shares, user);

        uint256 userBalanceBefore = user.balance;

        // Complete withdraw
        vm.prank(user);
        vault.completeWithdraw(user);

        // Verify request is cleared
        (,,,,,, uint256 sharesAfter) = vault.userWithdrawRequest(user);
        assertEq(sharesAfter, 0, "Shares should be 0 after completion");
        assertEq(user.balance, userBalanceBefore + 100, "Solver fee of 100 should be transferred to user");
    }

    function testCompleteWithdrawWithSolver() public {
        // Setup solver fee
        setFees(0, 0, 0, 100);

        // Create withdraw request with solver enabled
        vm.startPrank(user);
        vault.withdraw{value: 100}(WITHDRAW_AMOUNT, user, user, MAX_LOSS, true);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        uint256 solverBalanceBefore = solver.balance;

        // Complete withdraw as solver
        vm.prank(solver);
        vault.completeWithdraw(user);

        assertEq(solver.balance - solverBalanceBefore, 100, "Solver should receive fee");
    }

    function testCompleteWithdrawWithLossUnderMaxLoss() public {
        uint256 userBalance = token.balanceOf(user);

        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process first update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Update rate with small loss (4% loss)
        vm.startPrank(strategist);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS - 400, BASIS_POINTS), 0, 0);
        vm.stopPrank();

        // Should complete successfully as loss is under max
        vm.prank(user);
        vault.completeWithdraw(user);

        // Verify withdraw happened
        assertEq(
            token.balanceOf(user) - userBalance, WITHDRAW_AMOUNT * 96 / 100, "User should receive assets with 4% loss"
        );
    }

    function testCompleteWithdrawWithLossOverMaxLoss() public {
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Get initial shares
        (,,,,,, uint256 initialShares) = vault.userWithdrawRequest(user);

        // Process first update with a 1% withdraw fee (100 basis points)
        uint32 withdrawFee = 100; // 1%
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, withdrawFee, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Update rate with big loss (6% loss)
        uint256 newRate = ONE_SHARE.mulDiv(BASIS_POINTS - 600, BASIS_POINTS); // 94% of original rate (6% loss)
        vm.startPrank(strategist);
        vault.update(newRate, withdrawFee, 0);
        vm.stopPrank();

        uint256 userSharesBefore = vault.balanceOf(user);

        // Calculate expected refunded shares (initialShares - 1% fee)
        uint256 expectedRefundShares = initialShares.mulDiv(BASIS_POINTS - withdrawFee, BASIS_POINTS);

        // First try without vm.expectEmit to see what event is actually emitted
        vm.prank(user);
        vault.completeWithdraw(user);

        // Now check the actual values in assertions
        assertEq(
            vault.balanceOf(user),
            userSharesBefore + expectedRefundShares,
            "Shares should be refunded minus withdraw fee"
        );

        assertLt(
            vault.balanceOf(user),
            userSharesBefore + initialShares,
            "Refunded shares should be less than initial shares"
        );

        assertEq(
            vault.balanceOf(user) - userSharesBefore,
            expectedRefundShares,
            "Refunded shares should match expected amount after fee"
        );
    }

    function testCannotCompleteBeforeLockupPeriod() public {
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Try to complete before lockup period (should fail)
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.WithdrawNotClaimable.selector);
        vault.completeWithdraw(user);
        vm.stopPrank();
    }

    function testCannotCompleteWithUnauthorizedUser() public {
        // Create withdraw request without solver
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Try to complete with unauthorized user
        vm.prank(solver);
        vm.expectRevert(ValenceVault.SolverNotAllowed.selector);
        vault.completeWithdraw(user);
    }

    function testCompleteWithdrawWhenPaused() public {
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Pause vault
        vm.prank(owner);
        vault.pause();

        // Should fail when paused
        vm.expectRevert(ValenceVault.VaultIsPaused.selector);
        vault.completeWithdraw(user);
    }

    function testSolverFeeTransferFailure() public {
        // Setup solver fee
        setFees(0, 0, 0, 100);

        // Create withdraw request with solver enabled
        vm.startPrank(user);
        vault.withdraw{value: 100}(WITHDRAW_AMOUNT, user, user, MAX_LOSS, true);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Create a contract that rejects ETH transfers
        MockRejectingContract rejectingContract = new MockRejectingContract();
        vm.deal(address(rejectingContract), 1 ether);

        // Try to complete withdraw with rejecting contract as solver
        vm.prank(address(rejectingContract));
        vm.expectRevert(ValenceVault.SolverFeeTransferFailed.selector);
        vault.completeWithdraw(user);
    }

    function testBatchWithdrawSameUpdate() public {
        setFees(0, 0, 0, 100);

        // All users request withdraw before first update
        for (uint256 i = 0; i < users.length; i++) {
            vm.prank(users[i]);
            vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[i], users[i], MAX_LOSS, true); // Allow solver completion
        }

        // Process update for all withdraws
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 100, WITHDRAW_AMOUNT * users.length); // 1% fee
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Complete all withdraws in batch
        vm.prank(solver);
        vault.completeWithdraws(users);

        // Verify all withdraws from same update completed correctly
        for (uint256 i = 0; i < users.length; i++) {
            (,,,,,, uint256 shares) = vault.userWithdrawRequest(users[i]);
            assertEq(shares, 0, "Withdraw request should be cleared");
        }
    }

    function testBatchWithdrawDifferentUpdates() public {
        setFees(0, 0, 0, 100);

        // First user requests withdraw
        vm.prank(users[0]);
        vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[0], users[0], MAX_LOSS, true);

        // First update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 100, WITHDRAW_AMOUNT); // 1% fee
        vm.stopPrank();

        vm.warp(vm.getBlockTimestamp() + 1 days);

        // Second user requests withdraw after first update
        vm.prank(users[1]);
        vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[1], users[1], MAX_LOSS, true);

        // Second update with different rate and fee
        vm.startPrank(strategist);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS - 200, BASIS_POINTS), 200, WITHDRAW_AMOUNT); // 2% loss and 2% fee
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Complete withdraws in batch
        vm.prank(solver);
        vault.completeWithdraws(users);

        // Verify withdraws from different updates
        (,,,,,, uint256 shares0) = vault.userWithdrawRequest(users[0]);
        (,,,,,, uint256 shares1) = vault.userWithdrawRequest(users[1]);
        assertEq(shares0, 0, "First user withdraw should be cleared");
        assertEq(shares1, 0, "Second user withdraw should be cleared");
    }

    function testBatchWithdrawWithLossExceedingMax() public {
        setFees(0, 0, 0, 100);

        // All users request withdraw
        for (uint256 i = 0; i < users.length; i++) {
            vm.prank(users[i]);
            vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[i], users[i], MAX_LOSS, true); // Allow solver completion
        }

        // Update with significant loss (10%)
        vm.startPrank(strategist);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS - 1000, BASIS_POINTS), 0, WITHDRAW_AMOUNT * users.length);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        // Record initial share balances
        uint256[] memory initialShares = new uint256[](users.length);
        for (uint256 i = 0; i < users.length; i++) {
            initialShares[i] = vault.balanceOf(users[i]);
        }

        // Complete withdraws
        vm.prank(solver);
        vault.completeWithdraws(users);

        // Verify all users got refunded due to excessive loss
        for (uint256 i = 0; i < users.length; i++) {
            uint256 currentShares = vault.balanceOf(users[i]);
            assertTrue(currentShares > initialShares[i], "User should have refunded shares");
        }
    }

    function testBatchWithdrawWithSolverFees() public {
        // Setup solver fee
        setFees(0, 0, 0, 100);

        // Users request withdraws with solver enabled
        for (uint256 i = 0; i < users.length; i++) {
            vm.prank(users[i]);
            vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[i], users[i], MAX_LOSS, true);
        }

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT * users.length);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);

        uint256 solverBalanceBefore = solver.balance;

        // Complete withdraws
        vm.prank(solver);
        vault.completeWithdraws(users);

        // Verify solver received combined fees
        assertEq(solver.balance - solverBalanceBefore, 100 * users.length, "Solver should receive total fees");
    }

    function testBatchWithdrawMixedClaimTimes() public {
        // All users request withdraw
        for (uint256 i = 0; i < users.length; i++) {
            vm.prank(users[i]);
            vault.withdraw(WITHDRAW_AMOUNT, users[i], users[i], MAX_LOSS, true); // Allow solver completion
        }

        // Process update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 0, WITHDRAW_AMOUNT * users.length);
        vm.stopPrank();

        // Fast forward only partially
        vm.warp(vm.getBlockTimestamp() + 2 days); // Not enough time for 3 day lockup

        // Try to complete all withdraws
        vm.prank(solver);
        vault.completeWithdraws(users);

        // Verify all withdraws are still pending due to lockup
        for (uint256 i = 0; i < users.length; i++) {
            (,,,,,, uint256 shares) = vault.userWithdrawRequest(users[i]);
            assertTrue(shares > 0, "Withdraw should still be pending");
        }
    }

    function testMultipleWithdrawsWithVaryingLossThresholds() public {
        // Setup users with different loss thresholds
        uint32[] memory maxLosses = new uint32[](3);
        maxLosses[0] = 300; // 3% max loss
        maxLosses[1] = 500; // 5% max loss
        maxLosses[2] = 1000; // 10% max loss

        // Setup solver fee
        setFees(0, 0, 0, 100);

        // Track initial balances
        uint256[] memory initialBalances = new uint256[](3);
        for (uint256 i = 0; i < 3; i++) {
            initialBalances[i] = token.balanceOf(users[i]);
        }

        // Users request withdraws with different loss thresholds
        for (uint256 i = 0; i < 3; i++) {
            vm.prank(users[i]);
            vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[i], users[i], maxLosses[i], true);
        }

        // Process initial update
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, 100, WITHDRAW_AMOUNT * 3); // Set 1% withdraw fee
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Update rate with 4% loss (total 5% with 1% fee)
        vm.startPrank(strategist);
        vault.update(ONE_SHARE.mulDiv(BASIS_POINTS - 400, BASIS_POINTS), 100, 0);
        vm.stopPrank();

        // Store users current share balances
        uint256[] memory sharesBefore = new uint256[](3);
        for (uint256 i = 0; i < 3; i++) {
            sharesBefore[i] = vault.balanceOf(users[i]);
        }

        // Complete all withdraws
        address[] memory targetUsers = new address[](3);
        for (uint256 i = 0; i < 3; i++) {
            targetUsers[i] = users[i];
        }

        vm.prank(solver);
        vault.completeWithdraws(targetUsers);

        // Verify results:
        // First user (3% max loss) - should have gotten refund as 5% loss > 3% threshold
        (,,,,,, uint256 shares0) = vault.userWithdrawRequest(users[0]);
        assertEq(shares0, 0, "First user request should be cleared");
        assertTrue(vault.balanceOf(users[0]) > sharesBefore[0], "First user should have received refunded shares");

        // Second user (5% max loss) - borderline case, should still withdraw
        (,,,,,, uint256 shares1) = vault.userWithdrawRequest(users[1]);
        assertEq(shares1, 0, "Second user request should be cleared");
        assertTrue(token.balanceOf(users[1]) > initialBalances[1], "Second user should have received assets");

        // Third user (10% max loss) - should withdraw successfully
        (,,,,,, uint256 shares2) = vault.userWithdrawRequest(users[2]);
        assertEq(shares2, 0, "Third user request should be cleared");
        assertTrue(token.balanceOf(users[2]) > initialBalances[2], "Third user should have received assets");

        // Verify the exact amounts for users who successfully withdrew
        uint256 withdrawRate = ONE_SHARE.mulDiv(BASIS_POINTS - 500, BASIS_POINTS); // 96% rate - 1% fee
        uint256 expectedWithdrawAmount = WITHDRAW_AMOUNT.mulDiv(withdrawRate, ONE_SHARE);

        for (uint256 i = 1; i < 3; i++) {
            // Skip first user who got refunded
            assertEq(
                token.balanceOf(users[i]) - initialBalances[i],
                expectedWithdrawAmount,
                "Withdrawn amount should match exactly"
            );
        }
    }

    function testWithdrawFundsReturnToDepositOnMaxLossExceeded() public {
        console.log("Starting test...");

        // Setup initial state
        uint32 withdrawFee = 100; // 1% fee

        console.log("Initial balances:");
        console.log("Withdraw Account:", token.balanceOf(address(withdrawAccount)));
        console.log("Deposit Account:", token.balanceOf(address(depositAccount)));
        console.log("User token balance:", token.balanceOf(user));
        console.log("User vault shares:", vault.balanceOf(user));

        // Track initial balances
        uint256 withdrawAccountBalanceBefore = token.balanceOf(address(withdrawAccount));
        uint256 depositAccountBalanceBefore = token.balanceOf(address(depositAccount));

        console.log("Creating withdraw request...");
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Get request details after creation
        (,,,,,, uint256 sharesToWithdraw) = vault.userWithdrawRequest(user);
        console.log("Withdraw request created with shares:", sharesToWithdraw);

        console.log("Processing first update...");
        // Process first update to transfer funds to withdraw account
        vm.startPrank(strategist);
        vault.update(ONE_SHARE, withdrawFee, WITHDRAW_AMOUNT);
        vm.stopPrank();

        console.log("After first update:");
        console.log("Withdraw Account:", token.balanceOf(address(withdrawAccount)));
        console.log("Deposit Account:", token.balanceOf(address(depositAccount)));

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        console.log("Updating rate with loss...");
        // Update rate with 6% loss (exceeds max loss of 5%)
        uint256 newRate = ONE_SHARE.mulDiv(BASIS_POINTS - 600, BASIS_POINTS);
        uint256 loss = WITHDRAW_AMOUNT.mulDiv(600, BASIS_POINTS);
        console.log("New rate:", newRate);

        vm.startPrank(strategist);
        vault.update(newRate, withdrawFee, 0);
        vm.stopPrank();

        // Calculate expected refund
        console.log("Calculating refund...");
        console.log("Shares to withdraw:", sharesToWithdraw);
        console.log("Withdraw fee:", withdrawFee);
        console.log("BASIS_POINTS:", BASIS_POINTS);

        uint256 refundShares = sharesToWithdraw.mulDiv(BASIS_POINTS - withdrawFee, BASIS_POINTS, Math.Rounding.Floor);
        console.log("Refund shares calculated:", refundShares);

        console.log("Completing withdraw...");
        // Complete withdraw (should fail due to max loss and refund)
        vm.prank(user);
        vault.completeWithdraw(user);

        console.log("Final balances:");
        console.log("Withdraw Account:", token.balanceOf(address(withdrawAccount)));
        console.log("Deposit Account:", token.balanceOf(address(depositAccount)));
        console.log("User shares:", vault.balanceOf(user));

        // Verify changes in account balances
        assertEq(
            depositAccountBalanceBefore - token.balanceOf(address(depositAccount)),
            loss,
            "Deposit account should receive refunded assets"
        );

        // The withdraw account should still only have the "loss" in it,
        // In test the loss is not "lost" to the position
        assertEq(
            token.balanceOf(address(withdrawAccount)) - withdrawAccountBalanceBefore,
            loss,
            "Withdraw account balance should decrease by refunded amount"
        );

        // Verify user received refunded shares
        uint256 userSharesAfterRefund = vault.balanceOf(user);
        assertEq(
            userSharesAfterRefund,
            INITIAL_DEPOSIT_AMOUNT - WITHDRAW_AMOUNT + refundShares,
            "User should have received correct amount of refunded shares"
        );
    }
}

contract MockRejectingContract {
    receive() external payable {
        revert("ETH transfer rejected");
    }
}
