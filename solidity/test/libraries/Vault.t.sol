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

    // Fee of 500 basis points (5%)
    uint256 constant FEE_BPS = 500;

    function setUp() public {
        vm.warp(5000);
        vm.roll(100);

        vm.startPrank(owner);
        token = new MockERC20("Test Token", "TEST");
        depositAccount = new BaseAccount(owner, new address[](0));
        withdrawAccount = new BaseAccount(owner, new address[](0));

        // Setup fee configuration
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: 0
        });

        // Setup vault configuration
        ValenceVault.VaultConfig memory config = ValenceVault.VaultConfig({
            depositAccount: depositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: 0,
            maxWithdrawFee: 2000,
            fees: feeConfig
        });

        vault = new ValenceVault(
            owner,
            abi.encode(config),
            address(token),
            "Valence Vault Token",
            "VVT"
        );
        depositAccount.approveLibrary(address(vault));
        withdrawAccount.approveLibrary(address(vault));
        vm.stopPrank();

        // Setup initial state
        vm.startPrank(owner);
        token.mint(user, 100_000_000_000);
        vm.stopPrank();

        vm.startPrank(user);
        token.approve(address(vault), type(uint256).max);
        vm.stopPrank();
    }

    function setFee(uint256 fee) public {
        vm.startPrank(owner);
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: fee
        });

        (
            BaseAccount _depositAccount,
            BaseAccount _withdrawAccount,
            address _strategist,
            uint256 depositCap,
            uint256 maxWithdrawFee,

        ) = vault.config();

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            _depositAccount,
            _withdrawAccount,
            _strategist,
            depositCap,
            maxWithdrawFee,
            feeConfig
        );

        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();
    }

    function testUpdateConfig() public {
        vm.startPrank(owner);
        BaseAccount newDepositAccount = new BaseAccount(
            owner,
            new address[](0)
        );

        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: 0
        });

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            newDepositAccount,
            withdrawAccount,
            strategist,
            5000,
            2000,
            feeConfig
        );

        vault.updateConfig(abi.encode(newConfig));
        (BaseAccount depAcc, , , , , ) = vault.config();
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
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: 0
        });
        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount,
            strategist,
            5000, // 5000 token cap
            2000,
            feeConfig
        );
        vault.updateConfig(abi.encode(newConfig));

        vm.stopPrank();

        vm.startPrank(user);

        uint256 preBalance = token.balanceOf(user);

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
        assertEq(token.balanceOf(address(user)), preBalance - 5000);
        assertEq(vault.balanceOf(address(user)), 5000);

        vm.stopPrank();
    }

    function testMintCap() public {
        vm.startPrank(owner);
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: 0
        });
        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount,
            strategist,
            5000, // 5000 token cap
            2000,
            feeConfig
        );
        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();

        vm.startPrank(user);

        uint256 preBalance = token.balanceOf(user);

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
        assertEq(token.balanceOf(address(user)), preBalance - 5000);
        assertEq(vault.balanceOf(address(user)), 5000);

        vm.stopPrank();
    }

    function testFeeCalculationHelpers() public {
        setFee(FEE_BPS);
        uint256 depositAmount = 1000 ether;

        // Test deposit fee calculation
        uint256 expectedFee = (depositAmount * FEE_BPS) / 10000;
        uint256 calculatedFee = vault.calculateDepositFee(depositAmount);
        assertEq(
            calculatedFee,
            expectedFee,
            "Deposit fee calculation mismatch"
        );

        // Test mint fee calculation
        uint256 sharesToMint = 950 ether; // Should require 1000 ether input for 5% fee
        (uint256 grossAssets, uint256 fee) = vault.calculateMintFee(
            sharesToMint
        );

        // Verify the gross assets and fee
        assertEq(fee, expectedFee, "Mint fee calculation mismatch");
        assertEq(
            grossAssets,
            depositAmount,
            "Gross assets calculation mismatch"
        );
    }

    function testDepositAndMintEquivalence() public {
        setFee(FEE_BPS);
        uint256 depositAmount = 1000;

        // Test deposit flow
        vm.startPrank(user);
        uint256 depositShares = vault.deposit(depositAmount, user);
        uint256 depositFeeCollected = vault.feesOwedInAsset();

        // Reset fee counter and user balance for mint test
        vm.stopPrank();
        vm.startPrank(owner);
        token.mint(user, depositAmount); // Replenish user's tokens
        vm.stopPrank();

        // Calculate equivalent shares for mint
        uint256 expectedShares = depositShares;

        // Test mint flow
        vm.startPrank(user);
        uint256 mintAssets = vault.mint(expectedShares, user);
        uint256 mintFeeCollected = vault.feesOwedInAsset() -
            depositFeeCollected;

        // Verify equivalence
        assertEq(
            mintAssets,
            depositAmount,
            "Assets required for mint should match deposit amount"
        );
        assertEq(
            mintFeeCollected,
            depositFeeCollected,
            "Fees collected should be equal"
        );
        assertEq(
            vault.balanceOf(user),
            expectedShares * 2,
            "User should have received equal shares"
        );
    }

    function testFeesWithDifferentAmounts() public {
        setFee(FEE_BPS);
        uint256[] memory amounts = new uint256[](3);
        amounts[0] = 1000; // Small amount
        amounts[1] = 100_000; // Medium amount
        amounts[2] = 10_000_000; // Large amount

        for (uint256 i = 0; i < amounts.length; i++) {
            uint256 depositAmount = amounts[i];

            // Test deposit
            vm.startPrank(user);
            uint256 sharesByDeposit = vault.deposit(depositAmount, user);
            uint256 depositFee = vault.calculateDepositFee(depositAmount);

            // Test mint with equivalent shares
            (uint256 grossAssets, uint256 mintFee) = vault.calculateMintFee(
                sharesByDeposit
            );

            // Verify fee calculations match
            assertEq(
                depositFee,
                mintFee,
                string.concat(
                    "Fee mismatch for amount: ",
                    vm.toString(depositAmount)
                )
            );
            assertEq(
                grossAssets,
                depositAmount,
                string.concat(
                    "Gross assets mismatch for amount: ",
                    vm.toString(depositAmount)
                )
            );
            vm.stopPrank();
        }
    }

    function testZeroFeeCase() public {
        uint256 amount = 1000;

        // Verify no fees are charged
        assertEq(
            vault.calculateDepositFee(amount),
            0,
            "Should be no deposit fee"
        );

        (uint256 grossAssets, uint256 mintFee) = vault.calculateMintFee(amount);
        assertEq(mintFee, 0, "Should be no mint fee");
        assertEq(
            grossAssets,
            amount,
            "Gross assets should equal input with no fee"
        );

        // Verify actual operations
        vm.startPrank(user);
        uint256 preBalance = token.balanceOf(user);

        uint256 shares = vault.deposit(amount, user);
        assertEq(shares, amount, "Should get equal shares with no fee");
        assertEq(
            token.balanceOf(user),
            preBalance - amount,
            "Should transfer exact amount"
        );
        assertEq(vault.feesOwedInAsset(), 0, "Should collect no fees");
        vm.stopPrank();
    }

    function testPauseUnpauseAndPermissions() public {
        // Test only strategist can pause
        vm.startPrank(user);
        vm.expectRevert(
            abi.encodeWithSelector(
                ValenceVault.OnlyOwnerOrStrategistAllowed.selector
            )
        );
        vault.pause(true);
        vm.stopPrank();

        // Test pause functionality
        vm.startPrank(strategist);
        vault.pause(true);
        assertTrue(vault.paused());
        vm.stopPrank();

        // Test deposits blocked when paused
        vm.startPrank(user);
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.VaultIsPaused.selector)
        );
        vault.deposit(1000, user);
        vm.stopPrank();

        // Test unpause and deposit
        vm.startPrank(owner);
        vault.pause(false);
        assertFalse(vault.paused());
        vm.stopPrank();

        vm.startPrank(user);
        vault.deposit(1000, user);
        assertEq(vault.totalAssets(), 1000);
        vm.stopPrank();
    }

    function testUpdateRateAndFee() public {
        vm.startPrank(strategist);

        // Test valid update
        vault.update(11000, 500); // 1.1x rate and 5% fee
        assertEq(vault.redemptionRate(), 11000);
        assertEq(vault.positionWithdrawFee(), 500);

        // Test deposit after rate change
        vm.stopPrank();
        vm.startPrank(user);
        uint256 depositAmount = 1000;
        vault.deposit(depositAmount, user);
        // With 1.1x rate, 1000 assets should give ~909 shares (1000 * 10000 / 11000)
        assertEq(vault.balanceOf(user), 909);
        vm.stopPrank();
    }

    function testUpdateRateAndFeeRestrictions() public {
        vm.startPrank(user);
        // Test non-strategist cannot update
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.OnlyStrategistAllowed.selector)
        );
        vault.update(11000, 500);
        vm.stopPrank();

        vm.startPrank(strategist);
        // Test cannot set zero rate
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.InvalidRate.selector)
        );
        vault.update(0, 500);

        // Test cannot set fee above max
        vm.expectRevert(
            abi.encodeWithSelector(ValenceVault.InvalidWithdrawFee.selector)
        );
        vault.update(10000, 2100); // Above 20%
        vm.stopPrank();
    }

    function testUpdateEvents() public {
        vm.startPrank(strategist);

        vm.expectEmit(true, true, true, true);
        emit ValenceVault.RateUpdated(11000);
        vm.expectEmit(true, true, true, true);
        emit ValenceVault.WithdrawFeeUpdated(500);

        vault.update(11000, 500);
        vm.stopPrank();
    }
}
