// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";

contract ValenceVaultWithdrawTest is VaultHelper {
    event WithdrawRequested(
        uint256 indexed requestId,
        address indexed owner,
        address indexed receiver,
        uint256 shares,
        uint256 maxLossBps,
        bool solverEnabled
    );

    function setUp() public override {
        super.setUp();

        // Setup initial state - deposit some tokens
        vm.startPrank(user);
        vault.deposit(10000, user);
        vm.stopPrank();
    }

    function testBasicWithdraw() public {
        vm.startPrank(user);
        uint256 withdrawShares = 1000;
        uint256 preBalance = vault.balanceOf(user);

        uint64 requestId = vault.withdraw(
            withdrawShares,
            user,
            user,
            500, // 5% max loss
            false // no solver
        );

        // Verify request created correctly
        ValenceVault.WithdrawRequest memory request = vault.getRequest(
            requestId
        );
        assertEq(
            request.sharesAmount,
            withdrawShares,
            "Incorrect shares amount"
        );
        assertEq(request.owner, user, "Incorrect owner");
        assertEq(request.receiver, user, "Incorrect receiver");
        assertEq(request.maxLossBps, 500, "Incorrect maxLoss");
        assertEq(request.solverFee, 0, "Should have no solver fee");

        // Verify shares burned
        assertEq(
            vault.balanceOf(user),
            preBalance - withdrawShares,
            "Shares not burned"
        );

        vm.stopPrank();
    }

    function testWithdrawForOther() public {
        address receiver = makeAddr("receiver");
        vm.startPrank(user);

        uint64 requestId = vault.withdraw(
            1000,
            receiver,
            user,
            500,
            false
        );

        ValenceVault.WithdrawRequest memory request = vault.getRequest(
            requestId
        );
        assertEq(request.receiver, receiver, "Incorrect receiver");
        assertEq(request.owner, user, "Incorrect owner");

        vm.stopPrank();
    }

    function testWithdrawWithSolver() public {
        vm.startPrank(user);
        uint256 preBalance = token.balanceOf(user);

        uint64 requestId = vault.withdraw(
            1000,
            user,
            user,
            500,
            true
        );

        ValenceVault.WithdrawRequest memory request = vault.getRequest(
            requestId
        );
        (, , , , , , ValenceVault.FeeConfig memory fees) = vault.config();
        assertEq(
            request.solverFee,
            fees.solverCompletionFee,
            "Incorrect solver fee"
        );
        assertEq(
            token.balanceOf(user),
            preBalance - fees.solverCompletionFee,
            "Solver fee not charged"
        );

        vm.stopPrank();
    }

    function testWithdrawCount() public {
        vm.startPrank(user);

        uint256 maxWithdraws = vault.getMaxWithdraws();

        // Create max allowed withdraws
        for (uint256 i = 0; i < maxWithdraws; i++) {
            vault.withdraw(100, user, user, 500, false);
        }

        // Try to create one more
        vm.expectRevert(
            abi.encodeWithSelector(
                ValenceVault.TooManyWithdraws.selector,
                maxWithdraws,
                maxWithdraws
            )
        );
        vault.withdraw(100, user, user, 500, false);

        vm.stopPrank();
    }

    function testWithdrawEmitsEvent() public {
        vm.startPrank(user);
        uint256 shares = 1000;
        uint256 maxLoss = 500;

        vm.expectEmit(true, true, true, true);
        emit WithdrawRequested(
            1,
            user,
            user,
            shares,
            maxLoss,
            false
        );

        vault.withdraw(shares, user, user, maxLoss, false);
        vm.stopPrank();
    }

    function testWithdrawWithoutAllowance() public {
        address other = makeAddr("other");
        vm.startPrank(other);

        vm.expectRevert(
            abi.encodeWithSelector(
                ValenceVault.InsufficientAllowance.selector,
                1000,
                0
            )
        );
        vault.withdraw(1000, other, user, 500, false);

        vm.stopPrank();
    }

    function testWithdrawWithAllowance() public {
        address spender = makeAddr("spender");
        vm.startPrank(user);
        vault.approve(spender, 1000);
        vm.stopPrank();

        vm.startPrank(spender);
        uint64 requestId = vault.withdraw(
            1000,
            spender,
            user,
            500,
            false
        );

        ValenceVault.WithdrawRequest memory request = vault.getRequest(
            requestId
        );
        assertEq(request.owner, user, "Incorrect owner");
        assertEq(request.receiver, spender, "Incorrect receiver");

        vm.stopPrank();
    }

    function testWithdrawToZeroAddress() public {
        vm.startPrank(user);

        vm.expectRevert(ValenceVault.InvalidReceiver.selector);
        vault.withdraw(1000, address(0), user, 500, false);

        vm.stopPrank();
    }

    function testWithdrawFromZeroAddress() public {
        vm.startPrank(user);

        vm.expectRevert(ValenceVault.InvalidOwner.selector);
        vault.withdraw(1000, user, address(0), 500, false);

        vm.stopPrank();
    }

    function testWithdrawZeroShares() public {
        vm.startPrank(user);

        vm.expectRevert(ValenceVault.InvalidShares.selector);
        vault.withdraw(0, user, user, 500, false);

        vm.stopPrank();
    }

    function testWithdrawInvalidMaxLoss() public {
        vm.startPrank(user);

        vm.expectRevert(ValenceVault.InvalidMaxLoss.selector);
        vault.withdraw(
            1000,
            user,
            user,
            BASIS_POINTS + 1,
            false
        );

        vm.stopPrank();
    }

    function testWithdrawWhenPaused() public {
        vm.prank(owner);
        vault.pause(true);

        vm.startPrank(user);
        vm.expectRevert(ValenceVault.VaultIsPaused.selector);
        vault.withdraw(1000, user, user, 500, false);
        vm.stopPrank();
    }

    function testWithdrawLinkedList() public {
        vm.startPrank(user);

        // Create multiple withdraws and verify linked list
        uint64 request1 = vault.withdraw(
            1000,
            user,
            user,
            500,
            false
        );
        uint64 request2 = vault.withdraw(
            1000,
            user,
            user,
            500,
            false
        );
        uint64 request3 = vault.withdraw(
            1000,
            user,
            user,
            500,
            false
        );

        // Verify the linked list
        assertEq(
            vault.userFirstRequestId(user),
            request3,
            "Incorrect first request"
        );

        ValenceVault.WithdrawRequest memory req3 = vault.getRequest(request3);
        assertEq(
            uint256(req3.nextId),
            request2,
            "Incorrect next pointer from request 3"
        );

        ValenceVault.WithdrawRequest memory req2 = vault.getRequest(request2);
        assertEq(
            uint256(req2.nextId),
            request1,
            "Incorrect next pointer from request 2"
        );

        ValenceVault.WithdrawRequest memory req1 = vault.getRequest(request1);
        assertEq(uint256(req1.nextId), 0, "Last request should point to 0");

        vm.stopPrank();
    }

    function testRequestIdIncrement() public {
        vm.startPrank(user);

        uint256 firstId = vault.withdraw(
            1000,
            user,
            user,
            500,
            false
        );
        uint256 secondId = vault.withdraw(
            1000,
            user,
            user,
            500,
            false
        );

        assertEq(secondId, firstId + 1, "Request IDs should increment");

        vm.stopPrank();
    }
}
