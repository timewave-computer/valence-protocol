// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {IERC20} from "@openzeppelin-contracts/token/ERC20/IERC20.sol";
import {console} from "forge-std/src/console.sol";

contract VaultCompleteWithdrawTest is VaultHelper {
    address solver;
    uint256 constant WITHDRAW_AMOUNT = 1000;
    uint64 constant MAX_LOSS = 500; // 5%

    event WithdrawCompleted(
        address indexed owner, address indexed receiver, uint256 assets, uint256 shares, address indexed executor
    );

    event WithdrawCancelled(address indexed owner, uint256 shares, uint256 currentLoss, uint256 maxAllowedLoss);

    function setUp() public override {
        super.setUp();
        solver = makeAddr("solver");
        vm.deal(solver, 1 ether);
        vm.deal(user, 1 ether);

        // Setup initial state - deposit some tokens to both accounts
        vm.startPrank(owner);
        token.mint(address(withdrawAccount), INITIAL_USER_BALANCE);
        vm.stopPrank();

        vm.startPrank(user);
        vault.deposit(10000, user);
        vm.stopPrank();
    }

    function testCompleteWithdrawBasicFlow() public {
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Get the request info for verification
        (uint256 shares,,,,,,) = vault.userWithdrawRequest(user);

        vm.expectEmit(true, true, true, true);
        emit WithdrawCompleted(user, user, WITHDRAW_AMOUNT, shares, user);

        // Complete withdraw
        vm.prank(user);
        vault.completeWithdraw(user);

        // Verify request is cleared
        (uint256 sharesAfter,,,,,,) = vault.userWithdrawRequest(user);
        assertEq(sharesAfter, 0, "Shares should be 0 after completion");
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
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

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
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Update rate with small loss (4% loss)
        vm.startPrank(strategist);
        vault.update(BASIS_POINTS * 96 / 100, 0, 0);
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
        (uint256 initialShares,,,,,,) = vault.userWithdrawRequest(user);

        // Process first update with a 1% withdraw fee (100 basis points)
        uint64 withdrawFee = 100; // 1%
        vm.startPrank(strategist);
        vault.update(BASIS_POINTS, withdrawFee, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Update rate with big loss (6% loss)
        uint256 newRate = BASIS_POINTS * 94 / 100; // 94% of original rate (6% loss)
        vm.startPrank(strategist);
        vault.update(newRate, withdrawFee, 0);
        vm.stopPrank();

        uint256 userSharesBefore = vault.balanceOf(user);

        // Calculate expected refunded shares (initialShares - 1% fee)
        uint256 expectedRefundShares = initialShares * (BASIS_POINTS - withdrawFee) / BASIS_POINTS;

        // Calculate expected loss in BPS
        uint256 originalWithdrawRate = BASIS_POINTS - withdrawFee;
        uint256 newWithdrawRate = newRate - withdrawFee;
        uint256 expectedLossBps = ((originalWithdrawRate - newWithdrawRate) * BASIS_POINTS) / originalWithdrawRate;

        // First try without vm.expectEmit to see what event is actually emitted
        vm.recordLogs();
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

    function testCannotCompleteNonExistentWithdraw() public {
        vm.expectRevert(ValenceVault.WithdrawRequestNotFound.selector);
        vault.completeWithdraw(user);
    }

    function testCannotCompleteBeforeLockupPeriod() public {
        // Create withdraw request
        vm.startPrank(user);
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        vm.stopPrank();

        // Process update
        vm.startPrank(strategist);
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
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
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

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
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Pause vault
        vm.prank(owner);
        vault.pause(true);

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
        vault.update(BASIS_POINTS, 0, WITHDRAW_AMOUNT);
        vm.stopPrank();

        // Fast forward past lockup period
        vm.warp(block.timestamp + 3 days + 1);

        // Create a contract that rejects ETH transfers
        MockRejectingContract rejectingContract = new MockRejectingContract();
        vm.deal(address(rejectingContract), 1 ether);

        // Try to complete withdraw with rejecting contract as solver
        vm.prank(address(rejectingContract));
        vm.expectRevert(ValenceVault.SolverFeeTransferFailed.selector);
        vault.completeWithdraw(user);
    }
}

contract MockRejectingContract {
    receive() external payable {
        revert("ETH transfer rejected");
    }
}
