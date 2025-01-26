// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";

abstract contract ValenceVaultWithdrawBaseTest is VaultHelper {
    event WithdrawRequested(
        uint256 indexed requestId,
        address indexed owner,
        address indexed receiver,
        uint256 shares,
        uint256 maxLossBps,
        bool solverEnabled
    );

    function setUp() public virtual override {
        super.setUp();
        // Setup initial state - deposit some tokens
        vm.startPrank(user);
        vault.deposit(10000, user);
        vm.stopPrank();

        vm.deal(user, 10000);
    }

    // Abstract function to be implemented by child contracts
    function executeWithdraw(uint256 amount, address receiver, address owner, uint64 maxLossBps, bool allowSolver, ValenceVault.FeeConfig memory fees)
        internal
        virtual
        returns (uint64);

    // Shared tests
    function testBasicRequest() public {
        vm.startPrank(user);
        uint256 amount = 1000;
        uint256 preBalance = vault.balanceOf(user);

        uint64 requestId = executeWithdraw(amount, user, user, 500, false, defaultFees());

        ValenceVault.WithdrawRequest memory request = vault.getRequest(requestId);
        // For withdraw, shares will be converted. For redeem, it's direct.
        // Child tests will verify the specific share amount
        assertEq(request.owner, user, "Incorrect owner");
        assertEq(request.receiver, user, "Incorrect receiver");
        assertEq(request.maxLossBps, 500, "Incorrect maxLoss");
        assertEq(request.solverFee, 0, "Should have no solver fee");
        assertTrue(vault.balanceOf(user) < preBalance, "Shares not burned");

        vm.stopPrank();
    }

    function testWithSolver() public {
        ValenceVault.FeeConfig memory fees = setFees(0, 0, 0, 100);

        vm.startPrank(user);
        uint256 preBalance = user.balance;

        uint64 requestId = executeWithdraw(1000, user, user, 500, true, fees);

        ValenceVault.WithdrawRequest memory request = vault.getRequest(requestId);
        assertEq(request.solverFee, fees.solverCompletionFee, "Incorrect solver fee");
        assertEq(user.balance, preBalance - fees.solverCompletionFee, "Solver fee not charged");

        vm.stopPrank();
    }

    function testRequestCount() public {
        vm.startPrank(user);
        uint256 maxWithdraws = vault.getMaxWithdraws();

        for (uint256 i = 0; i < maxWithdraws; i++) {
            executeWithdraw(100, user, user, 500, false, defaultFees());
        }

        vm.expectRevert(abi.encodeWithSelector(ValenceVault.TooManyWithdraws.selector, maxWithdraws, maxWithdraws));
        executeWithdraw(100, user, user, 500, false, defaultFees());

        vm.stopPrank();
    }

    // Common validation tests
    function testToZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidReceiver.selector);
        executeWithdraw(1000, address(0), user, 500, false, defaultFees());
        vm.stopPrank();
    }

    function testFromZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidOwner.selector);
        executeWithdraw(1000, user, address(0), 500, false, defaultFees());
        vm.stopPrank();
    }

    function testZeroAmount() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidAmount.selector);
        executeWithdraw(0, user, user, 500, false, defaultFees());
        vm.stopPrank();
    }

    function testInvalidMaxLoss() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidMaxLoss.selector);
        executeWithdraw(1000, user, user, uint64(BASIS_POINTS + 1), false, defaultFees());
        vm.stopPrank();
    }

    function testWhenPaused() public {
        vm.prank(owner);
        vault.pause(true);

        vm.startPrank(user);
        vm.expectRevert(ValenceVault.VaultIsPaused.selector);
        executeWithdraw(1000, user, user, 500, false, defaultFees());
        vm.stopPrank();
    }

    function testRequestChaining() public {
        vm.startPrank(user);

        // Create first request
        uint64 firstId = executeWithdraw(1000, user, user, 500, false, defaultFees());

        // Create second request
        uint64 secondId = executeWithdraw(1000, user, user, 500, false, defaultFees());

        // Check chaining
        ValenceVault.WithdrawRequest memory secondRequest = vault.getRequest(secondId);

        assertEq(secondRequest.nextId, firstId);
        assertEq(vault.userFirstRequestId(user), secondId);

        vm.stopPrank();
    }

    function testRequestMapping() public {
        ValenceVault.FeeConfig memory fees = setFees(0, 0, 0, 100);

        vm.startPrank(user);

        // Create regular request
        uint64 userRequestId = executeWithdraw(1000, user, user, 500, false, defaultFees());

        // Create solver request
        uint64 solverRequestId = executeWithdraw(1000, user, user, 500, true, fees);

        // Check correct mapping storage
        ValenceVault.WithdrawRequest memory userRequest = vault.getRequest(userRequestId);
        ValenceVault.WithdrawRequest memory solverRequest = vault.getRequest(solverRequestId);

        assertEq(userRequest.owner, user);
        assertEq(solverRequest.owner, user);
        assertTrue(solverRequest.solverFee > 0);

        vm.stopPrank();
    }

    function testUpdateIdTracking() public {
        vm.startPrank(user);

        uint64 currentUpdateId = vault.currentUpdateId();
        uint64 requestId = executeWithdraw(1000, user, user, 500, false, defaultFees());

        ValenceVault.WithdrawRequest memory request = vault.getRequest(requestId);
        assertEq(request.updateId, currentUpdateId + 1);

        vm.stopPrank();
    }

    function testTotalAssetsToWithdrawAccumulation() public {
        vm.startPrank(user);

        uint256 initialTotal = vault.totalAssetsToWithdrawNextUpdate();
        uint256 withdrawAmount = 1000;

        executeWithdraw(withdrawAmount, user, user, 500, false, defaultFees());

        uint256 newTotal = vault.totalAssetsToWithdrawNextUpdate();
        assertEq(newTotal, initialTotal + withdrawAmount);

        vm.stopPrank();
    }
}

contract ValenceVaultWithdrawTest is ValenceVaultWithdrawBaseTest {
    function executeWithdraw(uint256 amount, address receiver, address owner, uint64 maxLossBps, bool allowSolver, ValenceVault.FeeConfig memory fees)
        internal
        override
        returns (uint64)
    {
        if (allowSolver && fees.solverCompletionFee > 0) {
            return vault.withdraw{value: fees.solverCompletionFee}(amount, receiver, owner, maxLossBps, allowSolver);
        } else {
            return vault.withdraw(amount, receiver, owner, maxLossBps, allowSolver);
        }
    }

    function testMaxWithdraw() public {
        vm.startPrank(user);
        uint256 maxAssets = vault.maxWithdraw(user);

        vm.expectRevert(
            abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxWithdraw.selector, user, maxAssets + 1, maxAssets)
        );
        vault.withdraw(maxAssets + 1, user, user, 500, false);
        vm.stopPrank();
    }
}

contract ValenceVaultRedeemTest is ValenceVaultWithdrawBaseTest {
    function executeWithdraw(uint256 amount, address receiver, address owner, uint64 maxLossBps, bool allowSolver, ValenceVault.FeeConfig memory fees)
        internal
        override
        returns (uint64)
    {
        if (allowSolver && fees.solverCompletionFee > 0) {
            return vault.redeem{value: fees.solverCompletionFee}(amount, receiver, owner, maxLossBps, allowSolver);
        } else {
            return vault.redeem(amount, receiver, owner, maxLossBps, allowSolver);
        }
    }

    function testMaxRedeem() public {
        vm.startPrank(user);
        uint256 maxShares = vault.maxRedeem(user);

        vm.expectRevert(
            abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxRedeem.selector, user, maxShares + 1, maxShares)
        );
        vault.redeem(maxShares + 1, user, user, 500, false);
        vm.stopPrank();
    }
}
