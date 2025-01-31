// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";

contract VaultConfigTest is VaultHelper {
    function testUpdateConfigWithNewValues() public {
        vm.startPrank(owner);

        // Create new deposit account and addresses
        BaseAccount newDepositAccount = new BaseAccount(owner, new address[](0));
        address newStrategist = makeAddr("newStrategist");
        address newStrategistFeeAccount = makeAddr("newStrategistFeeAccount");
        address newPlatformFeeAccount = makeAddr("newPlatformFeeAccount");

        // Create new config with all updated values
        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig({
            depositAccount: newDepositAccount,
            withdrawAccount: withdrawAccount,
            strategist: newStrategist,
            depositCap: 1000000,
            maxWithdrawFee: 1000, // 10%
            withdrawLockupPeriod: 2 days,
            fees: ValenceVault.FeeConfig({
                depositFeeBps: 100, // 1%
                platformFeeBps: 500, // 5%
                performanceFeeBps: 2000, // 20%
                solverCompletionFee: 0.1 ether
            }),
            feeDistribution: ValenceVault.FeeDistributionConfig({
                strategistAccount: newStrategistFeeAccount,
                platformAccount: newPlatformFeeAccount,
                strategistRatioBps: 4000 // 40%
            })
        });

        vault.updateConfig(abi.encode(newConfig));

        // Verify all config changes
        (
            BaseAccount updatedDepositAccount,
            BaseAccount updatedWithdrawAccount,
            address updatedStrategist,
            ValenceVault.FeeConfig memory updatedFees,
            ValenceVault.FeeDistributionConfig memory updatedFeeDistribution,
            uint256 updatedDepositCap,
            uint64 updatedLockupPeriod,
            uint32 updatedMaxWithdrawFee
        ) = vault.config();

        // Assert account updates
        assertEq(address(updatedDepositAccount), address(newDepositAccount), "Deposit account not updated");
        assertEq(address(updatedWithdrawAccount), address(withdrawAccount), "Withdraw account should not change");

        // Assert basic config updates
        assertEq(updatedStrategist, newStrategist, "Strategist not updated");
        assertEq(updatedDepositCap, 1000000, "Deposit cap not updated");
        assertEq(updatedMaxWithdrawFee, 1000, "Max withdraw fee not updated");
        assertEq(updatedLockupPeriod, 2 days, "Lockup period not updated");

        // Assert fee config updates
        assertEq(updatedFees.depositFeeBps, 100, "Deposit fee not updated");
        assertEq(updatedFees.platformFeeBps, 500, "Platform fee not updated");
        assertEq(updatedFees.performanceFeeBps, 2000, "Performance fee not updated");
        assertEq(updatedFees.solverCompletionFee, 0.1 ether, "Solver fee not updated");

        // Assert fee distribution updates
        assertEq(
            updatedFeeDistribution.strategistAccount, newStrategistFeeAccount, "Strategist fee account not updated"
        );
        assertEq(updatedFeeDistribution.platformAccount, newPlatformFeeAccount, "Platform fee account not updated");
        assertEq(updatedFeeDistribution.strategistRatioBps, 4000, "Strategist ratio not updated");

        vm.stopPrank();
    }

    function testCannotSetInvalidDepositAccount() public {
        vm.startPrank(owner);

        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.depositAccount = BaseAccount(payable(address(0)));

        vm.expectRevert(ValenceVault.InvalidDepositAccount.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }

    function testCannotSetInvalidWithdrawAccount() public {
        vm.startPrank(owner);

        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.withdrawAccount = BaseAccount(payable(address(0)));

        vm.expectRevert(ValenceVault.InvalidWithdrawAccount.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }

    function testCannotSetInvalidStrategist() public {
        vm.startPrank(owner);

        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.strategist = address(0);

        vm.expectRevert(ValenceVault.InvalidStrategist.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }

    function testCannotSetInvalidFees() public {
        vm.startPrank(owner);

        // Test deposit fee > 100%
        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.fees.depositFeeBps = BASIS_POINTS + 1;

        vm.expectRevert(ValenceVault.InvalidFeeConfiguration.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        // Test platform fee > 100%
        invalidConfig = _getConfig();
        invalidConfig.fees.platformFeeBps = BASIS_POINTS + 1;

        vm.expectRevert(ValenceVault.InvalidFeeConfiguration.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        // Test performance fee > 100%
        invalidConfig = _getConfig();
        invalidConfig.fees.performanceFeeBps = BASIS_POINTS + 1;

        vm.expectRevert(ValenceVault.InvalidFeeConfiguration.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }

    function testCannotSetInvalidFeeDistribution() public {
        vm.startPrank(owner);

        // Test strategist ratio > 100%
        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.feeDistribution.strategistRatioBps = BASIS_POINTS + 1;

        vm.expectRevert(ValenceVault.InvalidFeeDistribution.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        // Test zero platform account
        invalidConfig = _getConfig();
        invalidConfig.feeDistribution.platformAccount = address(0);

        vm.expectRevert(ValenceVault.InvalidPlatformAccount.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        // Test zero strategist account
        invalidConfig = _getConfig();
        invalidConfig.feeDistribution.strategistAccount = address(0);

        vm.expectRevert(ValenceVault.InvalidStrategistAccount.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }

    function testCannotSetInvalidWithdrawParameters() public {
        vm.startPrank(owner);

        // Test max withdraw fee > 100%
        ValenceVault.VaultConfig memory invalidConfig = _getConfig();
        invalidConfig.maxWithdrawFee = uint32(BASIS_POINTS + 1);

        vm.expectRevert(ValenceVault.InvalidMaxWithdrawFee.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        // Test zero lockup period
        invalidConfig = _getConfig();
        invalidConfig.withdrawLockupPeriod = 0;

        vm.expectRevert(ValenceVault.InvalidWithdrawLockupPeriod.selector);
        vault.updateConfig(abi.encode(invalidConfig));

        vm.stopPrank();
    }
}
