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
    }

    // Abstract function to be implemented by child contracts
    function executeWithdraw(uint256 amount, address receiver, address owner, uint256 maxLossBps, bool allowSolver)
        internal
        virtual
        returns (uint64);

    // Shared tests
    function testBasicRequest() public {
        vm.startPrank(user);
        uint256 amount = 1000;
        uint256 preBalance = vault.balanceOf(user);

        uint64 requestId = executeWithdraw(amount, user, user, 500, false);

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
        vm.startPrank(user);
        uint256 preBalance = token.balanceOf(user);

        uint64 requestId = executeWithdraw(1000, user, user, 500, true);

        ValenceVault.WithdrawRequest memory request = vault.getRequest(requestId);
        (,,,,,, ValenceVault.FeeConfig memory fees,) = vault.config();
        assertEq(request.solverFee, fees.solverCompletionFee, "Incorrect solver fee");
        assertEq(token.balanceOf(user), preBalance - fees.solverCompletionFee, "Solver fee not charged");

        vm.stopPrank();
    }

    function testRequestCount() public {
        vm.startPrank(user);
        uint256 maxWithdraws = vault.getMaxWithdraws();

        for (uint256 i = 0; i < maxWithdraws; i++) {
            executeWithdraw(100, user, user, 500, false);
        }

        vm.expectRevert(abi.encodeWithSelector(ValenceVault.TooManyWithdraws.selector, maxWithdraws, maxWithdraws));
        executeWithdraw(100, user, user, 500, false);

        vm.stopPrank();
    }

    // Common validation tests
    function testToZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidReceiver.selector);
        executeWithdraw(1000, address(0), user, 500, false);
        vm.stopPrank();
    }

    function testFromZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidOwner.selector);
        executeWithdraw(1000, user, address(0), 500, false);
        vm.stopPrank();
    }

    function testZeroAmount() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidShares.selector);
        executeWithdraw(0, user, user, 500, false);
        vm.stopPrank();
    }

    function testInvalidMaxLoss() public {
        vm.startPrank(user);
        vm.expectRevert(ValenceVault.InvalidMaxLoss.selector);
        executeWithdraw(1000, user, user, BASIS_POINTS + 1, false);
        vm.stopPrank();
    }

    function testWhenPaused() public {
        vm.prank(owner);
        vault.pause(true);

        vm.startPrank(user);
        vm.expectRevert(ValenceVault.VaultIsPaused.selector);
        executeWithdraw(1000, user, user, 500, false);
        vm.stopPrank();
    }
}

contract ValenceVaultWithdrawTest is ValenceVaultWithdrawBaseTest {
    function executeWithdraw(uint256 amount, address receiver, address owner, uint256 maxLossBps, bool allowSolver)
        internal
        override
        returns (uint64)
    {
        return vault.withdraw(amount, receiver, owner, maxLossBps, allowSolver);
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
    function executeWithdraw(uint256 amount, address receiver, address owner, uint256 maxLossBps, bool allowSolver)
        internal
        override
        returns (uint64)
    {
        return vault.redeem(amount, receiver, owner, maxLossBps, allowSolver);
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
