// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ERC4626, ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";

contract VaultTest is Test {
    ValenceVault vault;
    BaseAccount depositAccount;
    BaseAccount withdrawAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);
    address strategist = address(3);
    address user = address(4);

    // Events to test
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Deposit(
        address indexed sender,
        address indexed owner,
        uint256 assets,
        uint256 shares
    );

    function setUp() public {
        vm.warp(5000);
        vm.roll(100);

        vm.startPrank(owner);
        token = new MockERC20("Test Token", "TEST");
        depositAccount = new BaseAccount(owner, new address[](0));
        withdrawAccount = new BaseAccount(owner, new address[](0));

        ValenceVault.VaultConfig memory config = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount,
            strategist,
            0
        );

        vault = new ValenceVault(
            owner,
            processor,
            abi.encode(config),
            address(token),
            "Valence Vault Token",
            "VVT"
        );
        withdrawAccount.approveLibrary(address(vault));
        vm.stopPrank();

        // Setup initial state
        vm.startPrank(owner);
        token.mint(user, 10000);
        vm.stopPrank();

        vm.startPrank(user);
        token.approve(address(vault), type(uint256).max);
        vm.stopPrank();
    }

    function testUpdateConfig() public {
        vm.startPrank(owner);
        BaseAccount newDepositAccount = new BaseAccount(
            owner,
            new address[](0)
        );

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            newDepositAccount,
            withdrawAccount,
            strategist,
            5000
        );

        vault.updateConfig(abi.encode(newConfig));
        (BaseAccount depAcc, , , ) = vault.config();
        assertEq(address(depAcc), address(newDepositAccount));
        vm.stopPrank();
    }

    function testConvertToShares() public view {
        // Test 1:1 conversion (initial state)
        uint256 assets = 1000;
        uint256 expectedShares = assets;
        assertEq(vault.convertToShares(assets), expectedShares);

        // Test with small amounts
        assets = 1;
        expectedShares = 1;
        assertEq(vault.convertToShares(assets), expectedShares);

        // Test with large amounts
        assets = 1_000_000;
        expectedShares = 1_000_000;
        assertEq(vault.convertToShares(assets), expectedShares);
    }

    function testConvertToAssets() public view {
        // Test 1:1 conversion (initial state)
        uint256 shares = 1000;
        uint256 expectedAssets = shares;
        assertEq(vault.convertToAssets(shares), expectedAssets);

        // Test with small amounts
        shares = 1;
        expectedAssets = 1;
        assertEq(vault.convertToAssets(shares), expectedAssets);

        // Test with large amounts
        shares = 1_000_000;
        expectedAssets = 1_000_000;
        assertEq(vault.convertToAssets(shares), expectedAssets);
    }

    function testTotalAssets() public {
        // Test empty vault
        assertEq(vault.totalAssets(), 0);

        // Test with deposits
        vm.startPrank(user);
        vault.deposit(1000, user);
        assertEq(vault.totalAssets(), 1000);

        vault.deposit(500, user);
        assertEq(vault.totalAssets(), 1500);
        vm.stopPrank();
    }

    function testTotalSupplyZero() public view {
        assertEq(vault.totalSupply(), 0);
        assertEq(vault.totalAssets(), 0);
    }

    function testDeposit() public {
        vm.startPrank(user);

        uint256 depositAmount = 1000;
        uint256 preBalance = token.balanceOf(user);

        vault.deposit(depositAmount, user);

        assertEq(token.balanceOf(address(depositAccount)), depositAmount);
        assertEq(token.balanceOf(user), preBalance - depositAmount);
        assertEq(vault.balanceOf(user), depositAmount);
        assertEq(vault.totalSupply(), depositAmount);

        vm.stopPrank();
    }

    function testDepositCap() public {
        vm.startPrank(owner);

        // Set a deposit cap
        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount,
            strategist,
            5000 // 5000 token cap
        );
        vault.updateConfig(abi.encode(newConfig));

        vm.stopPrank();

        vm.startPrank(user);

        // Test partial deposit
        vault.deposit(3000, user);
        assertEq(vault.totalAssets(), 3000);

        // Test deposit up to cap
        vault.deposit(2000, user);
        assertEq(vault.totalAssets(), 5000);

        // Test deposit exceeding cap
        vm.expectRevert(
            abi.encodeWithSelector(
                ERC4626.ERC4626ExceededMaxDeposit.selector,
                user,
                1000,
                0
            )
        );
        vault.deposit(1000, user);

        // Make sure the deposit account receives the deposits
        assertEq(token.balanceOf(address(depositAccount)), 5000);
assertEq(vault.balanceOf(address(user)), 5000);

        vm.stopPrank();
    }

    function testMintCap() public {
        vm.startPrank(owner);
        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount,
            strategist,
            5000 // 5000 token cap
        );
        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();

        vm.startPrank(user);

        // Test partial mint
        vault.mint(3000, user);
        assertEq(vault.totalSupply(), 3000);

        // Test mint up to cap
        vault.mint(2000, user);
        assertEq(vault.totalSupply(), 5000);

        // Test mint exceeding cap
        vm.expectRevert(
            abi.encodeWithSelector(
                ERC4626.ERC4626ExceededMaxMint.selector,
                user,
                1000,
                0
            )
        );
        vault.mint(1000, user);

        assertEq(token.balanceOf(address(depositAccount)), 5000);
        assertEq(vault.balanceOf(address(user)), 5000);

        vm.stopPrank();
    }
}
