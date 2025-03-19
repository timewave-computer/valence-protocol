// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ERC4626Upgradeable} from "../../src/libraries/ValenceVault.sol";
import {IERC20Errors} from "@openzeppelin/contracts/interfaces/draft-IERC6093.sol";

contract ValenceVaultDepositTest is VaultHelper {
    function testBasicDeposit() public {
        vm.startPrank(user);
        uint256 depositAmount = 1000;
        uint256 preBalance = token.balanceOf(user);

        vault.deposit(depositAmount, user);

        assertEq(token.balanceOf(address(depositAccount)), depositAmount, "Deposit account balance incorrect");
        assertEq(token.balanceOf(user), preBalance - depositAmount, "User balance not decreased");
        assertEq(vault.balanceOf(user), depositAmount, "User did not receive shares");
        assertEq(vault.totalSupply(), depositAmount, "Total supply not updated");
        vm.stopPrank();
    }

    function testBasicMint() public {
        vm.startPrank(user);
        uint256 mintAmount = 1000;
        uint256 preBalance = token.balanceOf(user);

        vault.mint(mintAmount, user);

        assertEq(token.balanceOf(address(depositAccount)), mintAmount, "Deposit account balance incorrect");
        assertEq(token.balanceOf(user), preBalance - mintAmount, "User balance not decreased");
        assertEq(vault.balanceOf(user), mintAmount, "User did not receive shares");
        assertEq(vault.totalSupply(), mintAmount, "Total supply not updated");
        vm.stopPrank();
    }

    function testDepositWithCap() public {
        uint128 cap = 5000;
        setDepositCap(cap);

        vm.startPrank(user);

        // Test partial deposit
        vault.deposit(3000, user);
        assertEq(vault.totalAssets(), 3000, "First deposit failed");

        // Test deposit up to cap
        vault.deposit(2000, user);
        assertEq(vault.totalAssets(), cap, "Second deposit failed");

        // Test deposit exceeding cap
        vm.expectRevert(abi.encodeWithSelector(ERC4626Upgradeable.ERC4626ExceededMaxDeposit.selector, user, 1000, 0));
        vault.deposit(1000, user);
        vm.stopPrank();
    }

    function testMintWithCap() public {
        uint128 cap = 5000;
        setDepositCap(cap);

        vm.startPrank(user);

        // Test partial mint
        vault.mint(3000, user);
        assertEq(vault.totalSupply(), 3000, "First mint failed");

        // Test mint up to cap
        vault.mint(2000, user);
        assertEq(vault.totalSupply(), cap, "Second mint failed");

        // Test mint exceeding cap
        vm.expectRevert(abi.encodeWithSelector(ERC4626Upgradeable.ERC4626ExceededMaxMint.selector, user, 1000, 0));
        vault.mint(1000, user);
        vm.stopPrank();
    }

    function testMultipleDeposits() public {
        vm.startPrank(user);

        vault.deposit(1000, user);
        assertEq(vault.totalAssets(), 1000, "First deposit failed");

        vault.deposit(500, user);
        assertEq(vault.totalAssets(), 1500, "Second deposit failed");

        vault.deposit(2500, user);
        assertEq(vault.totalAssets(), 4000, "Third deposit failed");

        vm.stopPrank();
    }

    function testMultipleMints() public {
        vm.startPrank(user);

        vault.mint(1000, user);
        assertEq(vault.totalSupply(), 1000, "First mint failed");

        vault.mint(500, user);
        assertEq(vault.totalSupply(), 1500, "Second mint failed");

        vault.mint(2500, user);
        assertEq(vault.totalSupply(), 4000, "Third mint failed");

        vm.stopPrank();
    }

    function testDepositEmitsEvent() public {
        vm.startPrank(user);
        uint256 depositAmount = 1000;

        vm.expectEmit(true, true, true, true);
        emit Deposit(user, user, depositAmount, depositAmount);

        vault.deposit(depositAmount, user);
        vm.stopPrank();
    }

    function testDepositForOther() public {
        address receiver = makeAddr("receiver");
        vm.startPrank(user);
        uint256 depositAmount = 1000;
        uint256 preBalance = token.balanceOf(user);

        vault.deposit(depositAmount, receiver);

        assertEq(token.balanceOf(address(depositAccount)), depositAmount, "Deposit account balance incorrect");
        assertEq(token.balanceOf(user), preBalance - depositAmount, "User balance not decreased");
        assertEq(vault.balanceOf(receiver), depositAmount, "Receiver did not get shares");
        assertEq(vault.balanceOf(user), 0, "User should not have shares");
        vm.stopPrank();
    }

    function testMintForOther() public {
        address receiver = makeAddr("receiver");
        vm.startPrank(user);
        uint256 mintAmount = 1000;
        uint256 preBalance = token.balanceOf(user);

        vault.mint(mintAmount, receiver);

        assertEq(token.balanceOf(address(depositAccount)), mintAmount, "Deposit account balance incorrect");
        assertEq(token.balanceOf(user), preBalance - mintAmount, "User balance not decreased");
        assertEq(vault.balanceOf(receiver), mintAmount, "Receiver did not get shares");
        assertEq(vault.balanceOf(user), 0, "User should not have shares");
        vm.stopPrank();
    }

    function testDepositToZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(abi.encodeWithSelector(IERC20Errors.ERC20InvalidReceiver.selector, address(0)));
        vault.deposit(1000, address(0));
        vm.stopPrank();
    }

    function testMintToZeroAddress() public {
        vm.startPrank(user);
        vm.expectRevert(abi.encodeWithSelector(IERC20Errors.ERC20InvalidReceiver.selector, address(0)));
        vault.mint(1000, address(0));
        vm.stopPrank();
    }
}
