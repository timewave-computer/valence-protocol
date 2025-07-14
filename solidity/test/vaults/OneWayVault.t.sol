// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {OneWayVault} from "../../src/vaults/OneWayVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract OneWayVaultTest is Test {
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
                          INITIALIZATION TESTS
    //////////////////////////////////////////////////////////////*/

    function test_Initialization() public view {
        // Verify initial state
        assertEq(vault.owner(), owner);

        (
            BaseAccount initializedDepositAccount,
            address initializedStrategist,
            uint32 initializedDepositFeeBps,
            uint32 initializedWithdrawRateBps,
            uint32 initializedMaxRateIncrementBps,
            uint32 initializedMaxRateDecrementBps,
            uint64 initializedMinRateUpdateDelay,
            uint64 initializedMaxRateUpdateDelay,
            uint256 initializedDepositCap,
            OneWayVault.FeeDistributionConfig memory initializedFeeDistribution
        ) = vault.config();

        assertEq(address(initializedDepositAccount), address(depositAccount));
        assertEq(initializedStrategist, strategist);
        assertEq(initializedDepositFeeBps, depositFeeBps);
        assertEq(initializedWithdrawRateBps, withdrawFeeBps);
        assertEq(initializedMaxRateIncrementBps, maxRateIncrementBps);
        assertEq(initializedMaxRateDecrementBps, maxRateDecrementBps);
        assertEq(initializedMinRateUpdateDelay, minRateUpdateDelay);
        assertEq(initializedMaxRateUpdateDelay, maxRateUpdateDelay);
        assertEq(initializedDepositCap, depositCap);
        assertEq(initializedFeeDistribution.strategistAccount, strategistFeeReceiver);
        assertEq(initializedFeeDistribution.platformAccount, platformFeeReceiver);
        assertEq(initializedFeeDistribution.strategistRatioBps, strategistRatioBps);
        assertEq(vault.redemptionRate(), initialRate);
        assertEq(vault.totalAssets(), 0);
        assertEq(vault.totalSupply(), 0);
        assertEq(vault.asset(), address(underlyingToken));
    }

    function test_Initialization_ZeroRedemptionRate() public {
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
            feeDistribution: OneWayVault.FeeDistributionConfig({
                strategistAccount: strategistFeeReceiver,
                platformAccount: platformFeeReceiver,
                strategistRatioBps: strategistRatioBps
            })
        });

        OneWayVault vaultImpl = new OneWayVault();

        bytes memory initData = abi.encodeWithSelector(
            OneWayVault.initialize.selector,
            owner,
            abi.encode(vaultConfig),
            address(underlyingToken),
            "Vault Test Token",
            "vTST",
            0 // This should cause the revert
        );

        // Create proxy creation code
        bytes memory proxyCreationCode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(address(vaultImpl), initData));

        // Expect the revert during proxy creation
        vm.expectRevert("Starting redemption rate cannot be zero");

        address proxyAddress;
        assembly {
            proxyAddress := create2(0, add(proxyCreationCode, 0x20), mload(proxyCreationCode), 0)
        }
    }

    /*//////////////////////////////////////////////////////////////
                            DEPOSIT TESTS
    //////////////////////////////////////////////////////////////*/

    function test_Deposit() public {
        uint256 depositAmount = 10_000 * 10 ** 18;

        // Calculate expected values
        uint256 fee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 depositAfterFee = depositAmount - fee;

        // Record balances before
        uint256 user1BalanceBefore = underlyingToken.balanceOf(user1);
        uint256 depositAccountBalanceBefore = underlyingToken.balanceOf(address(depositAccount));

        // Deposit
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Check balances after
        assertEq(underlyingToken.balanceOf(user1), user1BalanceBefore - depositAmount);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), depositAccountBalanceBefore + depositAmount);

        // Check shares issued
        uint256 expectedShares = (depositAfterFee * 10 ** vault.decimals()) / initialRate;
        assertEq(vault.balanceOf(user1), expectedShares);

        // Check total assets and fees
        assertEq(vault.totalAssets(), depositAfterFee);
        assertEq(vault.feesAccruedInAsset(), fee);
    }

    function test_DepositWithZeroFee() public {
        // Update config to set zero deposit fee
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: 0,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        uint256 depositAmount = 10_000 * 10 ** 18;

        // Record balances before
        uint256 user1BalanceBefore = underlyingToken.balanceOf(user1);
        uint256 depositAccountBalanceBefore = underlyingToken.balanceOf(address(depositAccount));

        // Deposit
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Check balances after
        assertEq(underlyingToken.balanceOf(user1), user1BalanceBefore - depositAmount);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), depositAccountBalanceBefore + depositAmount);

        // Check shares issued (should be 1:1 with deposit since rate is 1:1 and no fee)
        uint256 expectedShares = depositAmount;
        assertEq(vault.balanceOf(user1), expectedShares);

        // Check total assets and fees
        assertEq(vault.totalAssets(), depositAmount);
        assertEq(vault.feesAccruedInAsset(), 0);
    }

    function test_DepositWithCap() public {
        // Update config to set a small deposit cap
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: 0,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: 5_000 * 10 ** 18, // 5k tokens
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Try to deposit more than the cap
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vm.expectRevert(); // Should revert with ERC4626ExceededMaxDeposit
        vault.deposit(depositAmount, user1);

        // Test deposit up to the cap
        depositAmount = 5_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Check maxDeposit is now 0
        assertEq(vault.maxDeposit(user1), 0);
    }

    function test_Mint() public {
        uint256 mintShares = 10_000 * 10 ** 18;

        // Calculate expected deposit amount including fee
        uint256 baseAssets = (mintShares * initialRate) / 10 ** vault.decimals();
        uint256 grossAssets = Math.mulDiv(baseAssets, BASIS_POINTS, BASIS_POINTS - depositFeeBps, Math.Rounding.Ceil);
        uint256 fee = grossAssets - baseAssets;

        // Record balances before
        uint256 user1BalanceBefore = underlyingToken.balanceOf(user1);
        uint256 depositAccountBalanceBefore = underlyingToken.balanceOf(address(depositAccount));

        // Mint
        vm.prank(user1);
        vault.mint(mintShares, user1);

        // Check balances after
        assertEq(underlyingToken.balanceOf(user1), user1BalanceBefore - grossAssets);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), depositAccountBalanceBefore + grossAssets);

        // Check shares issued
        assertEq(vault.balanceOf(user1), mintShares);

        // Check total assets and fees
        assertEq(vault.totalAssets(), baseAssets);
        assertEq(vault.feesAccruedInAsset(), fee);
    }

    /*//////////////////////////////////////////////////////////////
                              PAUSE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_ManualPauseAndUnpause() public {
        // Test pause by owner
        vm.prank(owner);
        vault.pause();

        (bool paused, bool pausedByOwner, bool pausedByStaleRate) = vault.vaultState();

        assertTrue(paused);
        assertTrue(pausedByOwner);
        assertFalse(pausedByStaleRate);

        // Try to deposit while paused
        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.deposit(1000 * 10 ** 18, user1);

        // Unpause by owner
        vm.prank(owner);
        vault.unpause();

        (paused, pausedByOwner, pausedByStaleRate) = vault.vaultState();

        assertFalse(paused);
        assertFalse(pausedByOwner);
        assertFalse(pausedByStaleRate);

        // Test pause by strategist
        vm.prank(strategist);
        vault.pause();

        (paused, pausedByOwner, pausedByStaleRate) = vault.vaultState();

        assertTrue(paused);
        assertFalse(pausedByOwner);
        assertFalse(pausedByStaleRate);

        // Try to unpause by strategist (should work since not paused by owner)
        vm.prank(strategist);
        vault.unpause();

        (paused, pausedByOwner, pausedByStaleRate) = vault.vaultState();

        assertFalse(paused);
        assertFalse(pausedByOwner);
        assertFalse(pausedByStaleRate);

        // Test pause by owner, then try to unpause by strategist (should fail)
        vm.prank(owner);
        vault.pause();

        vm.prank(strategist);
        vm.expectRevert("Only owner can unpause if paused by owner");
        vault.unpause();

        // Owner can still unpause
        vm.prank(owner);
        vault.unpause();
    }

    function test_PauseByNonAuthorized() public {
        // Try to pause as non-owner/strategist
        vm.prank(user1);
        vm.expectRevert("Only owner or strategist allowed");
        vault.pause();
    }

    function test_StaleRateTrigger() public {
        // Let's make a first deposit which will succeed and user will get shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);
        // Check that user has shares
        assertTrue(vault.balanceOf(user1) > 0);

        // Let's not update the rate for a long time
        vm.warp(block.timestamp + maxRateUpdateDelay + 1 days); // Warp past max delay
        // Now if user2 tries to deposit he will not get shares and vault will be paused
        vm.prank(user2);
        vm.expectEmit(false, false, false, true);
        emit PausedStateChanged(true);
        vault.deposit(depositAmount, user2);

        // Check that vault is paused
        (bool paused, bool pausedByOwner, bool pausedByStaleRate) = vault.vaultState();
        assertTrue(paused);
        assertFalse(pausedByOwner);
        assertTrue(pausedByStaleRate);
        // Check that user2 got no shares
        assertEq(vault.balanceOf(user2), 0);

        // Check that we can't withdraw or redeem while paused
        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.withdraw(1, "neutron123", user1);

        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.redeem(1, "neutron123", user1);

        // Depositing or minting while paused should also fail
        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.deposit(depositAmount, user1);

        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.mint(10_000, user1);

        // Only owner can unpause, strategist cant
        vm.prank(strategist);
        vm.expectRevert("Only owner can unpause if paused by stale rate");
        vault.unpause();

        // If no update has happened, even owner can't unpause
        vm.prank(owner);
        vm.expectRevert("Cannot unpause while rate is stale");
        vault.unpause();

        // Update the rate so that owner can unpause and users can deposit again
        uint256 newRate = initialRate * 2; // Double the rate
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(newRate);
        vault.update(newRate);

        // Owner can unpause
        vm.prank(owner);
        vm.expectEmit(false, false, false, true);
        emit PausedStateChanged(false);
        vault.unpause();
        // Check that vault is unpaused
        (paused, pausedByOwner, pausedByStaleRate) = vault.vaultState();
        assertFalse(paused);
        assertFalse(pausedByOwner);
        assertFalse(pausedByStaleRate);

        // Users can now deposit again
        vm.prank(user2);
        vault.deposit(depositAmount, user2);

        // User got the shares and vault didn't pause
        assertTrue(vault.balanceOf(user2) > 0);
        (paused,,) = vault.vaultState();
        assertFalse(paused);
    }

    function test_RateUpdatedDuringStaleRatePause() public {
        // Let's not update the rate for a long time
        vm.warp(block.timestamp + maxRateUpdateDelay + 1 days); // Warp past max delay

        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user2);
        vm.expectEmit(false, false, false, true);
        emit PausedStateChanged(true);
        vault.deposit(depositAmount, user2);

        // Do a rate update - this should update the rate but not unpause the vault
        uint256 newRate = initialRate * 2; // Double the rate
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(newRate);
        vault.update(newRate);

        // Check that vault is still paused
        (bool paused, bool pausedByOwner, bool pausedByStaleRate) = vault.vaultState();
        assertTrue(paused);
        assertFalse(pausedByOwner);
        assertTrue(pausedByStaleRate);
    }

    function test_CannotUpdateWhenManuallyPaused() public {
        // Let's make a first deposit which will succeed and user will get shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);
        // Check that user has shares
        assertTrue(vault.balanceOf(user1) > 0);

        // Pause the vault by owner
        vm.prank(owner);
        vault.pause();

        // Now try to update the rate - should revert
        uint256 newRate = initialRate * 2;
        vm.prank(strategist);
        vm.expectRevert("Vault is paused by owner or strategist");
        vault.update(newRate);

        // Unpause the vault
        vm.prank(owner);
        vault.unpause();

        // Now we can update the rate
        // Make enough time pass
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(newRate);
        vault.update(newRate);
    }

    /*//////////////////////////////////////////////////////////////
                          REDEMPTION RATE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_UpdateRate() public {
        // First do a deposit to have some assets and shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Update rate - can only be done by strategist
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        uint256 newRate = initialRate * 2; // Double the rate
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(newRate);
        vault.update(newRate);

        // Check new rate
        assertEq(vault.redemptionRate(), newRate);

        // Check that total assets has changed according to new rate
        // Include both deposit and fees in the calculation
        uint256 expectedAssets = depositAmount * 2; // Double the entire deposit including fees

        assertEq(vault.totalAssets(), expectedAssets);
    }

    function test_UpdateRateDistributesFees() public {
        // First do a deposit to generate some fees
        uint256 depositAmount = 100_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Calculate expected fee
        uint256 expectedFee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        assertEq(vault.feesAccruedInAsset(), expectedFee);

        // Update rate - should distribute fees
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        uint256 newRate = initialRate * 11 / 10; // Increase by 10%
        vm.prank(strategist);
        vm.expectEmit(true, true, false, false);
        emit FeesDistributed(strategistFeeReceiver, platformFeeReceiver, 0, 0); // Exact share values will vary
        vault.update(newRate);

        // Check that fees were distributed
        assertEq(vault.feesAccruedInAsset(), 0);

        // Check that strategist and platform received their shares
        assertTrue(vault.balanceOf(strategistFeeReceiver) > 0);
        assertTrue(vault.balanceOf(platformFeeReceiver) > 0);

        // Check distribution ratio (approximately 50/50 as per config)
        uint256 strategistShares = vault.balanceOf(strategistFeeReceiver);
        uint256 platformShares = vault.balanceOf(platformFeeReceiver);

        // Allow for some rounding error
        assertApproxEqRel(strategistShares, platformShares, 0.01e18); // 1% tolerance
    }

    function test_UpdateRateByNonStrategist() public {
        vm.prank(user1);
        vm.expectRevert("Only strategist allowed");
        vault.update(initialRate * 2);

        // Owner also cannot update rate
        vm.prank(owner);
        vm.expectRevert("Only strategist allowed");
        vault.update(initialRate * 2);
    }

    function test_CannotUpdateWhenMinDelayHasNotPassed() public {
        // First do a deposit to have some assets and shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Try to update rate before min delay has passed
        vm.prank(strategist);
        vm.expectRevert("Minimum rate update delay not passed");
        vault.update(initialRate * 2);

        // Warping just before the min delay should also fail
        vm.warp(block.timestamp + minRateUpdateDelay - 1 seconds);
        // Update should still fail
        vm.prank(strategist);
        vm.expectRevert("Minimum rate update delay not passed");
        vault.update(initialRate * 2);

        // Warp past the min delay
        vm.warp(block.timestamp + 1 seconds);
        // Now it should succeed
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(initialRate * 2);
        vault.update(initialRate * 2);
    }

    function test_CannotUpdateRedemptionRateOverMaxIncrement() public {
        // First do a deposit to have some assets and shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Try to update rate beyond max increment
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        uint256 tooHighRate = initialRate * 2 + 1; // More than 100% increase
        vm.prank(strategist);
        vm.expectRevert("Rate increase exceeds maximum allowed increment");
        vault.update(tooHighRate);

        // Check that rate is still the initial rate
        assertEq(vault.redemptionRate(), initialRate);

        // Now update to a valid rate within increment limit
        uint256 validRate = initialRate * 2; // Exactly 100% increase
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(validRate);
        vault.update(validRate);
    }

    function test_CannotUpdateRedemptionRateUnderMaxDecrement() public {
        // First do a deposit to have some assets and shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Try to update rate below max decrement
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        uint256 tooLowRate = (initialRate * 9000) / 10000 - 1; // 90% of initial rate minus 1 // More than 10% decrease
        vm.prank(strategist);
        vm.expectRevert("Rate decrease exceeds maximum allowed decrement");
        vault.update(tooLowRate);

        // Check that rate is still the initial rate
        assertEq(vault.redemptionRate(), initialRate);

        // Now update to a valid rate within decrement limit
        uint256 validRate = (initialRate * 9000) / 10000; // Exactly 10% decrease
        vm.prank(strategist);
        vm.expectEmit(false, false, false, true);
        emit RateUpdated(validRate);
        vault.update(validRate);
    }

    /*//////////////////////////////////////////////////////////////
                        WITHDRAWAL REQUEST TESTS
    //////////////////////////////////////////////////////////////*/

    function test_Redeem() public {
        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Calculate expected shares
        uint256 depositFee = vault.calculateDepositFee(depositAmount);
        uint256 depositAfterFee = depositAmount - depositFee;
        uint256 expectedShares = (depositAfterFee * 10 ** vault.decimals()) / initialRate;

        // Now redeem half the shares
        uint256 redeemShares = expectedShares / 2;
        string memory receiverAddress = "neutron1fqf5mprg3f5hytvzp3t7spmsum6rjrw80mq8zgkc0h6rxga0dtzqws3uu7";

        // Calculate expected values
        uint256 grossAssets = (redeemShares * initialRate) / 10 ** vault.decimals();
        uint256 expectedWithdrawFee = vault.calculateWithdrawalFee(grossAssets);
        uint256 netAssets = grossAssets - expectedWithdrawFee;
        uint256 expectedNetShares = (netAssets * 10 ** vault.decimals()) / initialRate;

        vm.prank(user1);
        vm.expectEmit(true, true, false, true);
        // Event should emit the net shares
        emit WithdrawRequested(0, user1, receiverAddress, expectedNetShares);
        vault.redeem(redeemShares, receiverAddress, user1);

        // Check that correct shares were burned (the full redeemShares amount)
        assertEq(vault.balanceOf(user1), expectedShares - redeemShares);

        // Check that withdrawal fee was added to fees owed
        assertEq(vault.feesAccruedInAsset(), depositFee + expectedWithdrawFee);

        // Check that withdraw request was created
        (uint64 id, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(id, 0);
        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, initialRate);
        // The withdrawal request stores NET shares (for cross-chain processing)
        assertEq(sharesAmount, expectedNetShares);

        // Check that request ID was incremented
        assertEq(vault.currentWithdrawRequestId(), 1);
    }

    function test_RedeemWithZeroWithdrawFee() public {
        // Update config to set zero withdraw fee
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: 0, // Zero withdraw fee
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        uint256 userShares = vault.balanceOf(user1);
        uint256 redeemShares = userShares / 2;
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        uint256 feesAccruedBefore = vault.feesAccruedInAsset();

        vm.prank(user1);
        vm.expectEmit(true, true, false, true);
        // With zero fees, burned shares = net shares in request
        emit WithdrawRequested(0, user1, receiverAddress, redeemShares);
        vault.redeem(redeemShares, receiverAddress, user1);

        // Check that no additional fees were added
        assertEq(vault.feesAccruedInAsset(), feesAccruedBefore);

        // Check that exact shares were burned (no fee deduction)
        assertEq(vault.balanceOf(user1), userShares - redeemShares);

        // Check withdrawal request has same shares (no fee deduction)
        (,,, uint256 sharesAmount,) = vault.withdrawRequests(0);
        assertEq(sharesAmount, redeemShares);
    }

    function test_Withdraw() public {
        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Calculate expected shares and assets after deposit fee
        uint256 depositFee = vault.calculateDepositFee(depositAmount);
        uint256 depositAfterFee = depositAmount - depositFee;

        // Now withdraw half the assets (gross amount)
        uint256 withdrawAssets = depositAfterFee / 2;
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        // Calculate expected withdrawal fee and net assets
        uint256 expectedWithdrawFee = vault.calculateWithdrawalFee(withdrawAssets);
        uint256 netAssets = withdrawAssets - expectedWithdrawFee;
        uint256 expectedNetShares = (netAssets * 10 ** vault.decimals()) / initialRate;

        vm.prank(user1);
        vm.expectEmit(true, true, false, true);
        // Event should emit the shares that were withdrawn
        emit WithdrawRequested(0, user1, receiverAddress, expectedNetShares);
        vault.withdraw(withdrawAssets, receiverAddress, user1);

        // Check that withdrawal fee was added to fees owed
        assertEq(vault.feesAccruedInAsset(), depositFee + expectedWithdrawFee);

        // Check that withdraw request was created
        (, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, initialRate);
        // The withdrawal request stores NET shares (for cross-chain processing)
        assertEq(sharesAmount, expectedNetShares);
    }

    function test_WithdrawWithAllowance() public {
        // First deposit to get some shares for user1
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);
        uint256 shares = vault.balanceOf(user1);

        // User1 approves user2 to spend half their shares
        uint256 approvedShares = shares / 2;
        vm.prank(user1);
        vault.approve(user2, approvedShares);

        // Calculate expected values for withdrawal
        uint256 grossAssets = (approvedShares * initialRate) / 10 ** vault.decimals();

        // Use the actual fee calculation function instead of manual calculation
        uint256 expectedWithdrawFee = vault.calculateWithdrawalFee(grossAssets);
        uint256 netAssets = grossAssets - expectedWithdrawFee;

        // Calculate expected shares to burn - this should be the FULL approved shares
        // because in redeem(), we burn the full shares amount that user specified
        uint256 expectedSharesToBurn = approvedShares;

        // User2 redeems on behalf of user1
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";
        vm.prank(user2);
        vault.redeem(approvedShares, receiverAddress, user1);

        // Check that correct shares were burned from user1
        // Should burn the full approved shares amount (including fee portion)
        assertEq(vault.balanceOf(user1), shares - expectedSharesToBurn);

        // Check allowance was spent appropriately
        // Allowance should be reduced by the approved shares amount
        assertEq(vault.allowance(user1, user2), 0);

        // Additional verification: check the withdrawal request
        (,, uint256 redemptionRate, uint256 sharesAmount,) = vault.withdrawRequests(0);

        // The withdrawal request should store net shares (for cross-chain processing)
        uint256 expectedNetShares = (netAssets * 10 ** vault.decimals()) / initialRate;
        assertEq(sharesAmount, expectedNetShares, "Withdrawal request should store net shares");

        // Verify redemption rate is correct
        assertEq(redemptionRate, initialRate, "Redemption rate should match current rate");
    }

    function test_RedeemInvalidParams() public {
        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        uint256 shares = vault.balanceOf(user1);
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        // Test zero owner address
        vm.prank(user1);
        vm.expectRevert("Owner of shares cannot be zero address");
        vault.redeem(shares / 2, receiverAddress, address(0));

        // Test empty receiver address
        vm.prank(user1);
        vm.expectRevert("Receiver cannot be empty");
        vault.redeem(shares / 2, "", user1);

        // Test zero shares amount
        vm.prank(user1);
        vm.expectRevert("Amount to withdraw cannot be zero");
        vault.redeem(0, receiverAddress, user1);

        // Test withdraw more than balance
        vm.prank(user1);
        vm.expectRevert(); // Should revert with ERC4626ExceededMaxRedeem
        vault.redeem(shares + 1, receiverAddress, user1);
    }

    /*//////////////////////////////////////////////////////////////
                        WITHDRAWAL FEE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_CalculateWithdrawalFee() public {
        uint256 withdrawAmount = 10_000 * 10 ** 18;

        // Expected fee calculation
        uint256 expectedFee = (withdrawAmount * withdrawFeeBps) / BASIS_POINTS;

        // Check calculated fee
        uint256 calculatedFee = vault.calculateWithdrawalFee(withdrawAmount);

        assertEq(calculatedFee, expectedFee);

        // Test with zero fee
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
            withdrawFeeBps: 0,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Check fee calculation with zero fee
        assertEq(vault.calculateWithdrawalFee(withdrawAmount), 0);
    }

    function test_WithdrawFeeDistribution() public {
        // First deposit to get some shares and generate deposit fees
        uint256 depositAmount = 100_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        uint256 userShares = vault.balanceOf(user1);

        // Redeem some shares to generate withdrawal fees
        uint256 redeemShares = userShares / 4; // Redeem 25%
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        vm.prank(user1);
        vault.redeem(redeemShares, receiverAddress, user1);

        // Calculate expected total fees (deposit + withdrawal)
        uint256 expectedDepositFee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 grossAssets = (redeemShares * initialRate) / 10 ** vault.decimals();
        uint256 expectedWithdrawFee = (grossAssets * withdrawFeeBps) / BASIS_POINTS;
        uint256 totalExpectedFees = expectedDepositFee + expectedWithdrawFee;

        assertEq(vault.feesAccruedInAsset(), totalExpectedFees);

        // Update rate to trigger fee distribution
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours); // Warp past min delay
        uint256 newRate = initialRate * 11 / 10; // 10% increase
        vm.prank(strategist);
        vault.update(newRate);

        // Check that fees were distributed
        assertEq(vault.feesAccruedInAsset(), 0);

        // Check that both deposit and withdrawal fees were distributed
        assertTrue(vault.balanceOf(strategistFeeReceiver) > 0);
        assertTrue(vault.balanceOf(platformFeeReceiver) > 0);

        // Verify the distribution includes both types of fees
        uint256 strategistShares = vault.balanceOf(strategistFeeReceiver);
        uint256 platformShares = vault.balanceOf(platformFeeReceiver);

        // Total fee shares should represent the total fees collected
        uint256 totalFeeShares = strategistShares + platformShares;
        uint256 expectedTotalFeeShares = (totalExpectedFees * 10 ** vault.decimals()) / initialRate;

        assertApproxEqAbs(totalFeeShares, expectedTotalFeeShares, 1); // Allow 1 wei rounding error
    }

    function test_DepositAndWithdrawBeforeUpdate() public {
        // Test the scenario where user deposits and withdraws before update
        uint256 depositAmount = 10_000 * 10 ** 18;

        // User deposits
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        uint256 depositFee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 userShares = vault.balanceOf(user1);

        // User immediately withdraws all shares
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";
        vm.prank(user1);
        vault.redeem(userShares, receiverAddress, user1);

        // Calculate expected withdrawal fee
        uint256 grossAssets = (userShares * initialRate) / 10 ** vault.decimals();
        uint256 expectedWithdrawFee = (grossAssets * withdrawFeeBps) / BASIS_POINTS;

        // Check total fees accumulated
        uint256 totalFees = depositFee + expectedWithdrawFee;
        assertEq(vault.feesAccruedInAsset(), totalFees);

        // User should have zero shares left
        assertEq(vault.balanceOf(user1), 0);

        // Net cost to user should be both fees
        uint256 netAssetsReceived = grossAssets - expectedWithdrawFee;
        uint256 totalCostToUser = depositAmount - netAssetsReceived;
        assertEq(totalCostToUser, totalFees);
    }

    /*//////////////////////////////////////////////////////////////
                          EDGE CASE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_FallbackFunction() public {
        // Try to call a non-existent function
        vm.prank(user1);
        (bool success,) = address(vault).call(abi.encodeWithSignature("nonExistentFunction()"));
        assertFalse(success);
    }

    function test_MaxDeposit() public {
        // Set a deposit cap
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
            depositCap: 100_000 * 10 ** 18,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Check max deposit
        assertEq(vault.maxDeposit(user1), 100_000 * 10 ** 18);

        // Deposit half the cap
        uint256 depositAmount = 50_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Check max deposit reduced
        uint256 fee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 depositAfterFee = depositAmount - fee;

        // Max deposit should be cap minus current assets
        assertEq(vault.maxDeposit(user1), 100_000 * 10 ** 18 - depositAfterFee);

        vaultConfig.depositCap = 0;

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Max deposit should be unlimited
        assertEq(vault.maxDeposit(user1), type(uint256).max);
    }

    /*//////////////////////////////////////////////////////////////
                          UPGRADE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_Upgrade() public {
        // Deploy new implementation
        OneWayVault newImpl = new OneWayVault();

        // Upgrade proxy
        vm.prank(owner);
        vault.upgradeToAndCall(address(newImpl), "");

        // Verify upgrade was successful by checking the implementation address
        // This is where the proxy contract stores its implementation address
        bytes32 implementationSlot = bytes32(uint256(keccak256("eip1967.proxy.implementation")) - 1);
        address implAddress;

        // Use vm.load from foundry if available
        bytes32 value = vm.load(address(vault), implementationSlot);
        implAddress = address(uint160(uint256(value)));

        assertEq(implAddress, address(newImpl));
    }

    function test_UpgradeNotOwner() public {
        // Deploy new implementation
        OneWayVault newImpl = new OneWayVault();

        // Try to upgrade as non-owner
        vm.prank(user1);
        vm.expectRevert();
        vault.upgradeToAndCall(address(newImpl), "");
    }

    /*//////////////////////////////////////////////////////////////
                        FEE CALCULATION TESTS
    //////////////////////////////////////////////////////////////*/

    function test_CalculateDepositFee() public {
        uint256 depositAmount = 10_000 * 10 ** 18;

        // Expected fee calculation
        uint256 expectedFee = (depositAmount * depositFeeBps) / BASIS_POINTS;

        // Check calculated fee
        uint256 calculatedFee = vault.calculateDepositFee(depositAmount);

        assertEq(calculatedFee, expectedFee);

        // Test with zero fee
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: 0,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Check fee calculation with zero fee
        assertEq(vault.calculateDepositFee(depositAmount), 0);
    }

    function test_CalculateMintFee() public {
        uint256 mintShares = 10_000 * 10 ** 18;

        // Expected calculations
        uint256 baseAssets = (mintShares * initialRate) / 10 ** vault.decimals();
        uint256 grossAssets = Math.mulDiv(baseAssets, BASIS_POINTS, BASIS_POINTS - depositFeeBps, Math.Rounding.Ceil);
        uint256 expectedFee = grossAssets - baseAssets;

        // Check calculated values
        (uint256 calculatedGrossAssets, uint256 calculatedFee) = vault.calculateMintFee(mintShares);

        assertEq(calculatedGrossAssets, grossAssets);
        assertEq(calculatedFee, expectedFee);

        // Test with zero fee
        OneWayVault.FeeDistributionConfig memory feeConfig = OneWayVault.FeeDistributionConfig({
            strategistAccount: strategistFeeReceiver,
            platformAccount: platformFeeReceiver,
            strategistRatioBps: strategistRatioBps
        });

        OneWayVault.OneWayVaultConfig memory vaultConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: 0,
            withdrawFeeBps: withdrawFeeBps,
            maxRateIncrementBps: maxRateIncrementBps,
            maxRateDecrementBps: maxRateDecrementBps,
            minRateUpdateDelay: minRateUpdateDelay,
            maxRateUpdateDelay: maxRateUpdateDelay,
            depositCap: depositCap,
            feeDistribution: feeConfig
        });

        vm.prank(owner);
        vault.updateConfig(abi.encode(vaultConfig));

        // Check fee calculation with zero fee
        (calculatedGrossAssets, calculatedFee) = vault.calculateMintFee(mintShares);
        assertEq(calculatedGrossAssets, baseAssets);
        assertEq(calculatedFee, 0);
    }

    /*//////////////////////////////////////////////////////////////
                        CONVERSION TESTS
    //////////////////////////////////////////////////////////////*/

    function test_ConvertToAssets() public {
        uint256 shares = 10_000 * 10 ** 18;

        // Expected assets at initial rate
        uint256 expectedAssets = (shares * initialRate) / 10 ** vault.decimals();

        // Check conversion
        uint256 assets = vault.previewRedeem(shares);
        assertEq(assets, expectedAssets);

        // Update rate and check again
        uint256 newRate = initialRate * 2; // Double the rate

        // First deposit to have some shares (required for update)
        vm.prank(user1);
        vault.deposit(10_000 * 10 ** 18, user1);

        // Make enough time pass to allow rate update
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);

        vm.prank(strategist);
        vault.update(newRate);

        // New expected assets with doubled rate
        uint256 newExpectedAssets = (shares * newRate) / 10 ** vault.decimals();

        // Check conversion with new rate
        assets = vault.previewRedeem(shares);
        assertEq(assets, newExpectedAssets);
    }

    function test_ConvertToShares() public {
        uint256 assets = 10_000 * 10 ** 18;

        // Expected shares at initial rate
        uint256 expectedShares = (assets * 10 ** vault.decimals()) / initialRate;

        // Check conversion
        uint256 shares = vault.previewDeposit(assets);
        assertEq(shares, expectedShares);

        // Update rate and check again
        uint256 newRate = initialRate * 2; // Double the rate

        // First deposit to have some shares (required for update)
        vm.prank(user1);
        vault.deposit(10_000 * 10 ** 18, user1);

        // Make enough time pass to allow rate update
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);

        vm.prank(strategist);
        vault.update(newRate);

        // New expected shares with doubled rate (should be half as many shares)
        uint256 newExpectedShares = (assets * 10 ** vault.decimals()) / newRate;

        // Check conversion with new rate
        shares = vault.previewDeposit(assets);
        assertEq(shares, newExpectedShares);
    }

    /*//////////////////////////////////////////////////////////////
                        INTEGRATION TESTS
    //////////////////////////////////////////////////////////////*/

    function test_FullLifecycle() public {
        // 1. Deposit from multiple users
        uint256 depositAmount1 = 50_000 * 10 ** 18;
        uint256 depositAmount2 = 30_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount1, user1);

        vm.prank(user2);
        vault.deposit(depositAmount2, user2);

        // 2. Update rate to simulate yield
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);
        uint256 newRate = initialRate * 12 / 10; // 20% increase
        vm.prank(strategist);
        vault.update(newRate);

        // 3. Check increased total assets
        uint256 totalDeposits = depositAmount1 + depositAmount2;
        uint256 expectedTotalAssets = totalDeposits * 12 / 10; // Include fees in asset calculation
        assertApproxEqAbs(vault.totalAssets(), expectedTotalAssets, 10);

        // 4. Withdrawal request from user1 (half their shares)
        uint256 user1SharesBefore = vault.balanceOf(user1);
        uint256 redeemShares = user1SharesBefore / 2;
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        vm.prank(user1);
        vault.redeem(redeemShares, receiverAddress, user1);

        // 5. Check user1 shares were burned correctly
        uint256 user1SharesAfter = vault.balanceOf(user1);

        // In redeem(), we burn the EXACT shares requested by user
        assertEq(user1SharesAfter, user1SharesBefore - redeemShares, "Should burn exact shares requested");

        // 6. Check withdrawal request was created with correct values
        (, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, newRate);

        // sharesAmount should be NET shares (less than redeemShares due to fees)
        assertTrue(sharesAmount > 0, "Should have positive shares in request");
        assertTrue(sharesAmount < redeemShares, "Net shares should be less than gross shares due to fees");

        // 7. Update rate again to simulate more yield and trigger fee distribution
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);
        uint256 newerRate = newRate * 13 / 10; // Additional 30% increase
        vm.prank(strategist);
        vault.update(newerRate);

        // 8. Check that accumulated fees (deposit + withdrawal) were distributed
        // Note: Fees might be distributed as shares or assets depending on implementation
        uint256 strategistBalance = vault.balanceOf(strategistFeeReceiver);
        uint256 platformBalance = vault.balanceOf(platformFeeReceiver);

        assertTrue(strategistBalance > 0 || platformBalance > 0, "Some fees should have been distributed");

        // 9. Make withdrawal request from user2 (all remaining shares)
        uint256 user2SharesBefore = vault.balanceOf(user2);

        vm.prank(user2);
        vault.redeem(user2SharesBefore, receiverAddress, user2);

        // Verify user2 shares were completely burned
        assertEq(vault.balanceOf(user2), 0, "User2 should have 0 shares after full redemption");

        // 10. Verify total supply consistency
        uint256 remainingUser1Shares = vault.balanceOf(user1);
        uint256 remainingUser2Shares = vault.balanceOf(user2); // Should be 0
        uint256 strategistShares = vault.balanceOf(strategistFeeReceiver);
        uint256 platformShares = vault.balanceOf(platformFeeReceiver);

        uint256 expectedTotalSupply = remainingUser1Shares + remainingUser2Shares + strategistShares + platformShares;
        assertEq(vault.totalSupply(), expectedTotalSupply, "Total supply should equal sum of all balances");

        // 11. Additional verification: Check that two withdrawal requests exist
        // First request (user1)
        (, address owner1,,, string memory receiver1) = vault.withdrawRequests(0);
        assertEq(owner1, user1);
        assertEq(receiver1, receiverAddress);

        // Second request (user2)
        (, address owner2,,, string memory receiver2) = vault.withdrawRequests(1);
        assertEq(owner2, user2);
        assertEq(receiver2, receiverAddress);
    }

    function test_RateImpactOnWithdrawals() public {
        // 1. Deposit from user
        uint256 depositAmount = 100_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // 2. Update rate to 2x
        vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);
        uint256 doubledRate = initialRate * 2;
        vm.prank(strategist);
        vault.update(doubledRate);

        // 3. Calculate expected assets including fees
        uint256 expectedAssets = depositAmount * 2; // Include fees in calculation

        // 4. Verify total assets reflects new rate
        assertEq(vault.totalAssets(), expectedAssets);

        // 5. Get current shares before withdrawal
        uint256 user1SharesBefore = vault.balanceOf(user1);

        // 6. Create withdrawal request for assets (accounting for withdrawal fee)
        uint256 withdrawAssets = expectedAssets / 4; // Withdraw 25% of total assets
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";

        vm.prank(user1);
        vault.withdraw(withdrawAssets, receiverAddress, user1);

        // 7. Get shares after withdrawal
        uint256 user1SharesAfter = vault.balanceOf(user1);
        uint256 sharesBurned = user1SharesBefore - user1SharesAfter;

        // 8. Check withdrawal request uses current rate
        (,, uint256 redemptionRate, uint256 sharesAmount,) = vault.withdrawRequests(0);
        assertEq(redemptionRate, doubledRate);

        // 9. Verify correct shares were burned (including fee)
        uint256 expectedSharesBurned = vault.previewWithdraw(withdrawAssets);
        assertEq(sharesBurned, expectedSharesBurned, "Incorrect shares burned");

        // 10. Verify withdrawal request stores NET shares (after fee deduction)
        uint256 withdrawalFee = vault.calculateWithdrawalFee(withdrawAssets);
        uint256 netAssets = withdrawAssets - withdrawalFee;
        uint256 expectedNetShares = vault.previewWithdraw(netAssets);
        assertEq(sharesAmount, expectedNetShares, "Incorrect net shares in withdrawal request");
    }

    /*//////////////////////////////////////////////////////////////
                        FUZZING TESTS
    //////////////////////////////////////////////////////////////*/

    function testFuzz_Deposit(uint256 amount) public {
        // Bound the fuzzing to reasonable amounts
        // Add a minimum threshold to avoid dust amounts
        uint256 minDepositAmount = 1000; // Minimum viable deposit (adjust based on token decimals)
        vm.assume(amount >= minDepositAmount && amount <= 1_000_000 * 10 ** 18);

        // Make sure user1 has enough tokens
        vm.startPrank(owner);
        underlyingToken.mint(user1, amount * 2);
        vm.stopPrank();

        vm.startPrank(user1);
        underlyingToken.approve(address(vault), amount);

        // Check if amount exceeds max deposit
        uint256 maxDeposit = vault.maxDeposit(user1);
        if (amount <= maxDeposit) {
            // Calculate expected shares before deposit
            uint256 fee = (amount * depositFeeBps) / BASIS_POINTS;
            uint256 assetsAfterFee = amount - fee;

            // Skip test if deposit would result in zero shares
            // This can happen with tiny amounts due to rounding
            if (assetsAfterFee > 0) {
                // Should succeed
                vault.deposit(amount, user1);

                // Verify user received shares
                assertTrue(vault.balanceOf(user1) > 0, "User should receive non-zero shares");

                // Verify deposit account received tokens
                assertTrue(
                    underlyingToken.balanceOf(address(depositAccount)) >= amount,
                    "Deposit account should receive tokens"
                );
            } else {
                // For extremely small deposits that would result in zero shares,
                // the behavior is implementation-dependent, so we skip
                vm.expectRevert();
                vault.deposit(amount, user1);
            }
        } else {
            // Should revert when exceeding max deposit
            vm.expectRevert();
            vault.deposit(amount, user1);
        }

        vm.stopPrank();
    }

    /*//////////////////////////////////////////////////////////////
                        STRESS TESTS
    //////////////////////////////////////////////////////////////*/

    function test_ManyDepositsAndWithdrawals() public {
        // Test with many deposits and withdrawals to check for accumulation errors
        uint256 userCount = 10;
        uint256 operationsPerUser = 5;
        uint256 baseAmount = 1_000 * 10 ** 18;

        // Create users and fund them
        address[] memory users = new address[](userCount);
        for (uint256 i = 0; i < userCount; i++) {
            users[i] = address(uint160(100 + i));
            vm.startPrank(owner);
            underlyingToken.mint(users[i], baseAmount * operationsPerUser);
            vm.stopPrank();
            vm.startPrank(users[i]);
            underlyingToken.approve(address(vault), type(uint256).max);
            vm.stopPrank();
        }

        // Perform operations
        for (uint256 op = 0; op < operationsPerUser; op++) {
            // Each user deposits
            for (uint256 i = 0; i < userCount; i++) {
                uint256 depositAmount = baseAmount * (i + 1) / userCount; // Varied amounts
                vm.prank(users[i]);
                vault.deposit(depositAmount, users[i]);
            }

            // Update rate occasionally to simulate yield accrual
            if (op > 0 && op % 2 == 0) {
                vm.warp(block.timestamp + minRateUpdateDelay + 1 hours);
                uint256 newRate = initialRate * (100 + op * 5) / 100; // Increase rate by 5% each update
                vm.prank(strategist);
                vault.update(newRate);
            }
        }

        // Calculate total gross deposits and expected fee range
        uint256 totalGrossDeposits = 0;
        for (uint256 op = 0; op < operationsPerUser; op++) {
            for (uint256 i = 0; i < userCount; i++) {
                totalGrossDeposits += baseAmount * (i + 1) / userCount;
            }
        }

        // Calculate a reasonable range for total assets accounting for both deposit and withdrawal fees
        uint256 minExpectedAssets = totalGrossDeposits * 85 / 100; // Allow for fees taken out
        uint256 maxExpectedAssets = totalGrossDeposits * 13 / 10; // Allow for yield and fees

        uint256 actualTotalAssets = vault.totalAssets();
        assertTrue(
            actualTotalAssets >= minExpectedAssets && actualTotalAssets <= maxExpectedAssets,
            "Total assets outside reasonable range"
        );

        // Each user redeems a portion (this will generate withdrawal fees)
        string memory receiverAddress = "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp";
        for (uint256 i = 0; i < userCount; i++) {
            uint256 userShares = vault.balanceOf(users[i]);
            if (userShares > 0) {
                uint256 redeemShares = userShares / 2; // Redeem half
                if (redeemShares > 0) {
                    uint256 userSharesBefore = vault.balanceOf(users[i]);
                    vm.prank(users[i]);
                    vault.redeem(redeemShares, receiverAddress, users[i]);
                    // Check some shares were burned (exact amount depends on withdrawal fees)
                    assertTrue(vault.balanceOf(users[i]) < userSharesBefore);
                }
            }
        }

        // Verify fee accumulation (should have both deposit and withdrawal fees)
        assertTrue(vault.feesAccruedInAsset() > 0, "Should have accumulated fees from deposits and withdrawals");
    }
}
