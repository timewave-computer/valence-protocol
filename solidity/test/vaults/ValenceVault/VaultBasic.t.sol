// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {BaseAccount} from "../../../src/accounts/BaseAccount.sol";
import {ValenceVault} from "../../../src/vaults/ValenceVault.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract VaultBasicTest is VaultHelper {
    using Math for uint256;

    function testInitialState() public view {
        assertEq(vault.redemptionRate(), ONE_SHARE, "Incorrect initial redemption rate");
        assertEq(vault.maxHistoricalRate(), ONE_SHARE, "Incorrect initial max historical rate");
        assertEq(vault.lastUpdateTotalShares(), 0, "Initial shares should be 0");
        assertEq(vault.feesOwedInAsset(), 0, "Initial fees owed should be 0");
        assertEq(vault.totalSupply(), 0, "Initial supply should be 0");
        assertEq(vault.totalAssets(), 0, "Initial assets should be 0");
    }

    function testConvertToShares() public view {
        // Test 1:1 conversion (initial state)
        uint256 assets = 1000;
        assertEq(vault.convertToShares(assets), assets, "Initial 1:1 conversion failed");

        // Test with small amounts
        assertEq(vault.convertToShares(1), 1, "Small amount conversion failed");

        // Test with large amounts
        assertEq(vault.convertToShares(1_000_000_000_000), 1_000_000_000_000, "Large amount conversion failed");
    }

    function testConvertToAssets() public view {
        // Test 1:1 conversion (initial state)
        uint256 shares = 1000;
        assertEq(vault.convertToAssets(shares), shares, "Initial 1:1 conversion failed");

        // Test with small amounts
        assertEq(vault.convertToAssets(1), 1, "Small amount conversion failed");

        // Test with large amounts
        assertEq(vault.convertToAssets(1_000_000_000_000), 1_000_000_000_000, "Large amount conversion failed");
    }

    function testConvertWithRateChanges() public {
        vm.startPrank(user);
        vault.deposit(10_000, user);
        vm.stopPrank();

        // Test rate increase (1.5x)
        uint256 increaseRate = (ONE_SHARE * 15) / 10; // 1.5x
        vm.startPrank(strategist);
        vault.update(increaseRate, 0, 0);
        vm.stopPrank();

        // Test asset to share conversion with increased rate
        uint256 assets = 1000;
        // 1000 assets * 10000 / 15000 = 666.666... shares (rounded down)
        uint256 expectedShares = assets.mulDiv(ONE_SHARE, increaseRate, Math.Rounding.Floor);
        assertEq(vault.convertToShares(assets), expectedShares, "Share conversion with increased rate failed");

        // Test share to asset conversion with increased rate
        uint256 shares = 1000;
        // 1000 shares * 15000 / 10000 = 1500 assets
        uint256 expectedAssets = shares.mulDiv(increaseRate, ONE_SHARE, Math.Rounding.Floor);
        assertEq(vault.convertToAssets(shares), expectedAssets, "Asset conversion with increased rate failed");
    }

    function testConvertWithDecreasedRate() public {
        vm.startPrank(user);
        vault.deposit(10_000, user);
        vm.stopPrank();

        // Test rate decrease (0.8x)
        uint256 decreaseRate = (ONE_SHARE * 8) / 10; // 0.8x
        vm.startPrank(strategist);
        vault.update(decreaseRate, 0, 0);
        vm.stopPrank();

        // Test asset to share conversion with decreased rate
        uint256 assets = 1000;
        // 1000 assets * 10000 / 8000 = 1250 shares
        uint256 expectedShares = assets.mulDiv(ONE_SHARE, decreaseRate, Math.Rounding.Floor);
        assertEq(vault.convertToShares(assets), expectedShares, "Share conversion with decreased rate failed");

        // Test share to asset conversion with decreased rate
        uint256 shares = 1000;
        // 1000 shares * 8000 / 10000 = 800 assets
        uint256 expectedAssets = shares.mulDiv(decreaseRate, ONE_SHARE, Math.Rounding.Floor);
        assertEq(vault.convertToAssets(shares), expectedAssets, "Asset conversion with decreased rate failed");
    }

    function testOnlyOwnerCanUpdateConfig() public {
        vm.startPrank(user);

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig({
            depositAccount: depositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: 5000,
            maxWithdrawFeeBps: MAX_WITHDRAW_FEE,
            withdrawLockupPeriod: ONE_DAY,
            fees: defaultFees(),
            feeDistribution: defaultDistributionFees()
        });

        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, user));
        vault.updateConfig(abi.encode(newConfig));

        vm.stopPrank();
    }

    function testVaultMetadata() public view {
        assertEq(vault.name(), "Valence Vault Token", "Incorrect vault name");
        assertEq(vault.symbol(), "VVT", "Incorrect vault symbol");
        assertEq(vault.decimals(), token.decimals(), "Incorrect decimals");
        assertEq(vault.asset(), address(token), "Incorrect asset address");
    }

    function testPause() public {
        // try pause with user, should fail
        vm.startPrank(user);
        vm.expectRevert(abi.encodeWithSelector(ValenceVault.OnlyOwnerOrStrategistAllowed.selector));
        vault.pause();
        vm.stopPrank();

        // strategist can pause and unpause
        vm.startPrank(strategist);
        vault.pause();
        vault.unpause();
        vm.stopPrank();

        // owner can pause and unpause
        vm.startPrank(owner);
        vault.pause();
        vault.unpause();
        vm.stopPrank();

        // Only owner can unpause if he paused it
        vm.startPrank(owner);
        vault.pause();
        vm.stopPrank();

        vm.startPrank(strategist);
        vm.expectRevert(abi.encodeWithSelector(ValenceVault.OnlyOwnerCanUnpause.selector));
        vault.unpause();
        vm.stopPrank();
    }
}
