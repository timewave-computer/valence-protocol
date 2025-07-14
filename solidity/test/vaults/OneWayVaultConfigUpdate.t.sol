// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {OneWayVault} from "../../src/vaults/OneWayVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract OneWayVaultConfigUpdate is Test {
    // Contracts
    OneWayVault vault;
    MockERC20 underlyingToken;
    BaseAccount depositAccount;

    // Test addresses
    address owner = address(1);
    address strategist = address(2);
    address user1 = address(3);
    address user2 = address(4);
    address platformFeeReceiver = address(5);
    address strategistFeeReceiver = address(6);

    // Config constants
    uint32 constant BASIS_POINTS = 10000;
    uint32 depositFeeBps = 100; // 1%
    uint32 withdrawFeeBps = 50; // 0.5%
    uint32 strategistRatioBps = 5000; // 50%
    uint128 depositCap = 1_000_000 * 10 ** 18; // 1 million tokens
    uint256 initialRate = 10 ** 18; // 1:1 initial rate
    uint64 maxRateUpdateDelay = 1 days;
    uint64 minRateUpdateDelay = 1 hours;
    uint32 maxRateIncrementBps = 10000; // 100%
    uint32 maxRateDecrementBps = 1000; // 10%

    // Events from the contract
    event PausedStateChanged(bool paused);
    event RateUpdated(uint256 newRate);
    event FeesDistributed(
        address indexed strategistAccount,
        address indexed platformAccount,
        uint256 strategistShares,
        uint256 platformShares
    );
    event WithdrawRequested(uint64 indexed id, address owner, string receiver, uint256 shares);
    event ConfigUpdated(address indexed updater, OneWayVault.OneWayVaultConfig newConfig);
    event Deposit(address indexed caller, address indexed owner, uint256 assets, uint256 shares);

    function setUp() public {
        // Deploy mock token
        vm.startPrank(owner);
        underlyingToken = new MockERC20("Test Token", "TST", 18);

        depositAccount = new BaseAccount(owner, new address[](0));

        // Create vault config
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        // Deploy vault implementation
        OneWayVault vaultImpl = new OneWayVault();

        // Deploy vault proxy
        bytes memory initData = abi.encodeWithSelector(
            OneWayVault.initialize.selector,
            owner,
            abi.encode(vaultConfig),
            address(underlyingToken),
            "Vault Test Token",
            "vTST",
            initialRate
        );

        // Create proxy via create2 and initialize in one step
        bytes memory proxyCreationCode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(address(vaultImpl), initData));

        address proxyAddress;
        assembly {
            proxyAddress := create2(0, add(proxyCreationCode, 0x20), mload(proxyCreationCode), 0)
        }

        vault = OneWayVault(payable(proxyAddress));

        // Distribute tokens for testing
        underlyingToken.mint(user1, 100_000 * 10 ** 18);
        underlyingToken.mint(user2, 100_000 * 10 ** 18);
        vm.stopPrank();

        // Approve tokens for vault
        vm.prank(user1);
        underlyingToken.approve(address(vault), type(uint256).max);

        vm.prank(user2);
        underlyingToken.approve(address(vault), type(uint256).max);
    }

    /*//////////////////////////////////////////////////////////////
                              CONFIG TESTS
    //////////////////////////////////////////////////////////////*/

    function test_UpdateConfig() public {
        // New config values
        address newStrategist = address(10);
        address newPlatformFeeReceiver = address(11);
        address newStrategistFeeReceiver = address(12);
        uint32 newDepositFeeBps = 200; // 2%
        uint32 newWithdrawRateBps = 75; // 0.75%
        uint32 newStrategistRatioBps = 6000; // 60%
        uint128 newDepositCap = 500_000 * 10 ** 18; // 500k tokens

        // Create new config
        OneWayVault.FeeDistributionConfig memory newFeeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: newStrategistFeeReceiver,
            platformAccount: newPlatformFeeReceiver,
            strategistRatioBps: newStrategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory newVaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount, // keep same deposit account
            strategist: newStrategist,
            depositFeeBps: newDepositFeeBps,
            withdrawFeeBps: newWithdrawRateBps,
            maxRateIncrementBps: maxRateIncrementBps, // keep same incrementBps
            maxRateDecrementBps: maxRateDecrementBps, // keep same decrementBps
            minRateUpdateDelay: minRateUpdateDelay, // keep same min time
            maxRateUpdateDelay: maxRateUpdateDelay, // keep same max time
            depositCap: newDepositCap,
            feeDistribution: newFeeConfig
        });

        // Update config (only owner can do this)
        vm.prank(owner);
        vault.updateConfig(abi.encode(newVaultConfig));

        // Verify config was updated
        (
            ,
            address updatedStrategist,
            uint32 updatedDepositFeeBps,
            uint32 updatedWithdrawRateBps,
            ,
            ,
            ,
            ,
            uint256 updatedDepositCap,
            OneWayVault.FeeDistributionConfig memory updatedFeeDistribution
        ) = vault.config();

        assertEq(updatedStrategist, newStrategist);
        assertEq(updatedDepositFeeBps, newDepositFeeBps);
        assertEq(updatedWithdrawRateBps, newWithdrawRateBps);
        assertEq(updatedDepositCap, newDepositCap);
        assertEq(updatedFeeDistribution.strategistAccount, newStrategistFeeReceiver);
        assertEq(updatedFeeDistribution.platformAccount, newPlatformFeeReceiver);
        assertEq(updatedFeeDistribution.strategistRatioBps, newStrategistRatioBps);
    }

    function test_UpdateConfig_NotOwner() public {
        // Create new config
        OneWayVault.OneWayVaultConfig memory newVaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: address(10),
            depositFeeBps: 200,
            withdrawFeeBps: 75,
            maxRateIncrementBps: 1,
            maxRateDecrementBps: 1,
            minRateUpdateDelay: 1 hours,
            maxRateUpdateDelay: 1 days,
            depositCap: 500_000 * 10 ** 18,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: address(11),
                platformAccount: address(12),
                strategistRatioBps: 6000
            })
        });

        // Try to update config as non-owner (should revert)
        vm.prank(user1);
        vm.expectRevert();
        vault.updateConfig(abi.encode(newVaultConfig));
    }

    function test_InvalidConfig() public {
        // Test zero address for deposit account
        OneWayVault.OneWayVaultConfig memory invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: BaseAccount(payable(address(0))),
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Deposit account cannot be zero address");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test zero address for strategist
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: address(0),
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Strategist cannot be zero address");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test deposit fee > 100%
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: 10001, // > 100%
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Deposit fee cannot exceed 100%");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test withdraw fee > 100%
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: 10001, // > 100%
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Withdraw fee cannot exceed 100%");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test strategist ratio > 100%
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: 10001 // > 100%
            })
        });

        vm.prank(owner);
        vm.expectRevert("Strategist account fee distribution ratio cannot exceed 100%");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test max rate update delay is 0
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: 0, // 0 delay
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Max rate update delay cannot be zero");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test max rate decrement cannot be more than 100%
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: 10001, // > 100%
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Max rate decrement cannot exceed 100%");
        vault.updateConfig(abi.encode(invalidConfig));

        // Test min update delay cannot be more than max update delay
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: 2 days, // More than max delay
            maxRateUpdateDelay: 1 days,
            depositCap: depositCap,
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        vm.prank(owner);
        vm.expectRevert("Minimum update delay cannot exceed maximum update delay");
        vault.updateConfig(abi.encode(invalidConfig));
    }
}
