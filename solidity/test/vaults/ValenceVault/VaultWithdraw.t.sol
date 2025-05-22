// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../../src/vaults/ValenceVault.sol";
import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";

abstract contract ValenceVaultWithdrawBaseTest is VaultHelper {
    event WithdrawRequested(
        address indexed owner, address indexed receiver, uint256 shares, uint256 maxLossBps, bool solverEnabled
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
    function executeWithdraw(
        uint256 amount,
        address receiver,
        address owner,
        uint32 maxLossBps,
        bool allowSolver,
        ValenceVault.FeeConfig memory fees
    ) internal virtual;

    // Shared tests
    function testBasicRequest() public {
        vm.startPrank(user);
        uint256 amount = 1000;
        uint256 preBalance = vault.balanceOf(user);

        executeWithdraw(amount, user, user, 500, false, defaultFees());

        (address _owner,, uint64 maxLossBps, address receiver,, uint256 solverFee,) = vault.userWithdrawRequest(user);

        // For withdraw, shares will be converted. For redeem, it's direct.
        // Child tests will verify the specific share amount
        assertEq(_owner, user, "Incorrect owner");
        assertEq(receiver, user, "Incorrect receiver");
        assertEq(maxLossBps, 500, "Incorrect maxLoss");
        assertEq(solverFee, 0, "Should have no solver fee");
        assertTrue(vault.balanceOf(user) < preBalance, "Shares not burned");

        vm.stopPrank();
    }

    function testWithSolver() public {
        ValenceVault.FeeConfig memory fees = setFees(0, 0, 0, 100);

        vm.startPrank(user);
        uint256 preBalance = user.balance;

        executeWithdraw(1000, user, user, 500, true, fees);

        (,,,,, uint256 solverFee,) = vault.userWithdrawRequest(user);
        assertEq(solverFee, fees.solverCompletionFee, "Incorrect solver fee");
        assertEq(user.balance, preBalance - fees.solverCompletionFee, "Solver fee not charged");

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
        executeWithdraw(1000, user, user, BASIS_POINTS + 1, false, defaultFees());
        vm.stopPrank();
    }

    function testWhenPaused() public {
        vm.prank(owner);
        vault.pause();

        vm.startPrank(user);
        vm.expectRevert(ValenceVault.VaultIsPaused.selector);
        executeWithdraw(1000, user, user, 500, false, defaultFees());
        vm.stopPrank();
    }

    function testUpdateIdTracking() public {
        vm.startPrank(user);

        ValenceVault.PackedValues memory packedValues = _getPackedValues();

        uint64 currentUpdateId = packedValues.currentUpdateId;
        executeWithdraw(1000, user, user, 500, false, defaultFees());
        (,,,, uint64 updateId,,) = vault.userWithdrawRequest(user);
        assertEq(updateId, currentUpdateId + 1);

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

    function testExcessiveSolverFee() public {
        ValenceVault.FeeConfig memory fees = setFees(0, 0, 0, 100);

        vm.startPrank(user);
        uint256 excessiveFee = fees.solverCompletionFee + 1;

        vm.deal(user, excessiveFee);
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.InvalidSolverFee.selector, excessiveFee, fees.solverCompletionFee)
        );
        vault.withdraw{value: excessiveFee}(1000, user, user, 500, true);

        vm.stopPrank();
    }

    function testInsufficientSolverFee() public {
        ValenceVault.FeeConfig memory fees = setFees(0, 0, 0, 100);

        vm.startPrank(user);
        uint256 insufficientFee = fees.solverCompletionFee - 1;

        vm.deal(user, insufficientFee);
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.InvalidSolverFee.selector, insufficientFee, fees.solverCompletionFee)
        );
        vault.withdraw{value: insufficientFee}(1000, user, user, 500, true);

        vm.stopPrank();
    }

    function testETHWithNonSolverRequest() public {
        vm.startPrank(user);

        vm.deal(user, 100);
        vm.expectRevert(ValenceVault.UnexpectedETH.selector);
        vault.withdraw{value: 100}(1000, user, user, 500, false);

        vm.stopPrank();
    }

    function testMultiUserWithdraws() public {
        address user2 = makeAddr("user2");
        vm.startPrank(owner);
        token.mint(user2, INITIAL_USER_BALANCE);
        vm.stopPrank();

        vm.startPrank(user2);
        token.approve(address(vault), type(uint256).max);
        vault.deposit(10000, user2);
        vm.stopPrank();

        // First user withdraws
        vm.startPrank(user);
        executeWithdraw(1000, user, user, 500, false, defaultFees());
        vm.stopPrank();

        // Second user should be able to withdraw
        vm.startPrank(user2);
        executeWithdraw(1000, user2, user2, 500, false, defaultFees());
        vm.stopPrank();

        assertTrue(vault.hasActiveWithdraw(user), "User 1 should have active withdraw");
        assertTrue(vault.hasActiveWithdraw(user2), "User 2 should have active withdraw");
    }

    function testWithdrawRequestPersistence() public {
        vm.startPrank(user);
        executeWithdraw(1000, user, user, 500, false, defaultFees());

        (,,,, uint64 updateId,, uint256 sharesAmount) = vault.userWithdrawRequest(user);

        // Fast forward but before update
        vm.warp(block.timestamp + 1 days);

        // Request should persist
        (,,,, uint64 newUpdateId,, uint256 newSharesAmount) = vault.userWithdrawRequest(user);
        assertEq(sharesAmount, newSharesAmount, "Request should persist until update");
        assertEq(updateId, newUpdateId, "Update ID should remain unchanged");

        vm.stopPrank();
    }

    function testOnlyOneWithdrawAllowed() public {
        vm.startPrank(user);

        executeWithdraw(1000, user, user, 500, false, defaultFees());

        vm.expectRevert(abi.encodeWithSelector(ValenceVault.WithdrawAlreadyExists.selector));
        executeWithdraw(1000, user, user, 500, false, defaultFees());

        vm.stopPrank();
    }
}

contract ValenceVaultWithdrawTest is ValenceVaultWithdrawBaseTest {
    function executeWithdraw(
        uint256 amount,
        address receiver,
        address owner,
        uint32 maxLossBps,
        bool allowSolver,
        ValenceVault.FeeConfig memory fees
    ) internal override {
        if (allowSolver && fees.solverCompletionFee > 0) {
            vault.withdraw{value: fees.solverCompletionFee}(amount, receiver, owner, maxLossBps, allowSolver);
        } else {
            vault.withdraw(amount, receiver, owner, maxLossBps, allowSolver);
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
    function executeWithdraw(
        uint256 amount,
        address receiver,
        address owner,
        uint32 maxLossBps,
        bool allowSolver,
        ValenceVault.FeeConfig memory fees
    ) internal override {
        if (allowSolver && fees.solverCompletionFee > 0) {
            vault.redeem{value: fees.solverCompletionFee}(amount, receiver, owner, maxLossBps, allowSolver);
        } else {
            vault.redeem(amount, receiver, owner, maxLossBps, allowSolver);
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
