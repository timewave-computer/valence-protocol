// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";

contract VaultBasicTest is VaultHelper {
    using Math for uint256;

    function testInitialState() public view {
        assertEq(
            vault.redemptionRate(),
            BASIS_POINTS,
            "Incorrect initial redemption rate"
        );
        assertEq(
            vault.maxHistoricalRate(),
            BASIS_POINTS,
            "Incorrect initial max historical rate"
        );
        assertEq(
            vault.lastUpdateTotalShares(),
            0,
            "Initial shares should be 0"
        );
        assertEq(
            vault.positionWithdrawFee(),
            0,
            "Initial withdraw fee should be 0"
        );
        assertEq(vault.feesOwedInAsset(), 0, "Initial fees owed should be 0");
        assertEq(vault.totalSupply(), 0, "Initial supply should be 0");
        assertEq(vault.totalAssets(), 0, "Initial assets should be 0");
    }

    function testConvertToShares() public view {
        // Test 1:1 conversion (initial state)
        uint256 assets = 1000;
        assertEq(
            vault.convertToShares(assets),
            assets,
            "Initial 1:1 conversion failed"
        );

        // Test with small amounts
        assertEq(vault.convertToShares(1), 1, "Small amount conversion failed");

        // Test with large amounts
        assertEq(
            vault.convertToShares(1_000_000),
            1_000_000,
            "Large amount conversion failed"
        );
    }

    function testConvertToAssets() public view {
        // Test 1:1 conversion (initial state)
        uint256 shares = 1000;
        assertEq(
            vault.convertToAssets(shares),
            shares,
            "Initial 1:1 conversion failed"
        );

        // Test with small amounts
        assertEq(vault.convertToAssets(1), 1, "Small amount conversion failed");

        // Test with large amounts
        assertEq(
            vault.convertToAssets(1_000_000),
            1_000_000,
            "Large amount conversion failed"
        );
    }

    function testConvertWithRateChanges() public {
        vm.startPrank(user);
        vault.deposit(10_000, user);
        vm.stopPrank();

        // Test rate increase (1.5x)
        uint256 increaseRate = (BASIS_POINTS * 15) / 10; // 1.5x
        vm.startPrank(strategist);
        vault.update(increaseRate, 0);
        vm.stopPrank();

        // Test asset to share conversion with increased rate
        uint256 assets = 1000;
        // 1000 assets * 10000 / 15000 = 666.666... shares (rounded down)
        uint256 expectedShares = assets.mulDiv(
            BASIS_POINTS,
            increaseRate,
            Math.Rounding.Floor
        );
        assertEq(
            vault.convertToShares(assets),
            expectedShares,
            "Share conversion with increased rate failed"
        );

        // Test share to asset conversion with increased rate
        uint256 shares = 1000;
        // 1000 shares * 15000 / 10000 = 1500 assets
        uint256 expectedAssets = shares.mulDiv(
            increaseRate,
            BASIS_POINTS,
            Math.Rounding.Floor
        );
        assertEq(
            vault.convertToAssets(shares),
            expectedAssets,
            "Asset conversion with increased rate failed"
        );
    }

    function testConvertWithDecreasedRate() public {
        vm.startPrank(user);
        vault.deposit(10_000, user);
        vm.stopPrank();

        // Test rate decrease (0.8x)
        uint256 decreaseRate = (BASIS_POINTS * 8) / 10; // 0.8x
        vm.startPrank(strategist);
        vault.update(decreaseRate, 0);
        vm.stopPrank();

        // Test asset to share conversion with decreased rate
        uint256 assets = 1000;
        // 1000 assets * 10000 / 8000 = 1250 shares
        uint256 expectedShares = assets.mulDiv(
            BASIS_POINTS,
            decreaseRate,
            Math.Rounding.Floor
        );
        assertEq(
            vault.convertToShares(assets),
            expectedShares,
            "Share conversion with decreased rate failed"
        );

        // Test share to asset conversion with decreased rate
        uint256 shares = 1000;
        // 1000 shares * 8000 / 10000 = 800 assets
        uint256 expectedAssets = shares.mulDiv(
            decreaseRate,
            BASIS_POINTS,
            Math.Rounding.Floor
        );
        assertEq(
            vault.convertToAssets(shares),
            expectedAssets,
            "Asset conversion with decreased rate failed"
        );
    }

    function testUpdateConfig() public {
        vm.startPrank(owner);

        // Create new deposit account
        BaseAccount newDepositAccount = new BaseAccount(
            owner,
            new address[](0)
        );
        uint256 newDepositCap = 5000;

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig({
            depositAccount: newDepositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: newDepositCap,
            maxWithdrawFee: MAX_WITHDRAW_FEE,
            withdrawLockupPeriod: ONE_DAY,
            fees: defaultFees(),
            feeDistribution: defaultDistributionFees()
        });

        vault.updateConfig(abi.encode(newConfig));

        // Verify config changes
        (
            BaseAccount updatedDepositAccount,
            BaseAccount updatedWithdrawAccount,
            address updatedStrategist,
            uint256 updatedDepositCap,
            ,
            ,
            ValenceVault.FeeConfig memory updatedFees,
        ) = vault.config();

        assertEq(
            address(updatedDepositAccount),
            address(newDepositAccount),
            "Deposit account not updated"
        );
        assertEq(
            address(updatedWithdrawAccount),
            address(withdrawAccount),
            "Withdraw account should not change"
        );
        assertEq(updatedStrategist, strategist, "Strategist should not change");
        assertEq(updatedDepositCap, newDepositCap, "Deposit cap not updated");
        assertEq(updatedFees.depositFeeBps, 0, "Fees should remain zero");
        vm.stopPrank();
    }

    function testOnlyOwnerCanUpdateConfig() public {
        vm.startPrank(user);

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig({
            depositAccount: depositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: 5000,
            maxWithdrawFee: MAX_WITHDRAW_FEE,
            withdrawLockupPeriod: ONE_DAY,
            fees: defaultFees(),
            feeDistribution: defaultDistributionFees()
        });

        vm.expectRevert(
            abi.encodeWithSelector(
                Ownable.OwnableUnauthorizedAccount.selector,
                user
            )
        );
        vault.updateConfig(abi.encode(newConfig));

        vm.stopPrank();
    }

    function testVaultMetadata() public view {
        assertEq(vault.name(), "Valence Vault Token", "Incorrect vault name");
        assertEq(vault.symbol(), "VVT", "Incorrect vault symbol");
        assertEq(vault.decimals(), token.decimals(), "Incorrect decimals");
        assertEq(vault.asset(), address(token), "Incorrect asset address");
    }
}
