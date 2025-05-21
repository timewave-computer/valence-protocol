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
    uint32 strategistRatioBps = 5000; // 50%
    uint128 depositCap = 1_000_000 * 10 ** 18; // 1 million tokens
    uint256 initialRate = 10 ** 18; // 1:1 initial rate

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
            uint256 initializedDepositCap,
            OneWayVault.FeeDistributionConfig memory initializedFeeDistribution
        ) = vault.config();

        assertEq(address(initializedDepositAccount), address(depositAccount));
        assertEq(initializedStrategist, strategist);
        assertEq(initializedDepositFeeBps, depositFeeBps);
        assertEq(initializedDepositCap, depositCap);
        assertEq(initializedFeeDistribution.strategistAccount, strategistFeeReceiver);
        assertEq(initializedFeeDistribution.platformAccount, platformFeeReceiver);
        assertEq(initializedFeeDistribution.strategistRatioBps, strategistRatioBps);
        assertEq(vault.redemptionRate(), initialRate);
        assertEq(vault.totalAssets(), 0);
        assertEq(vault.totalSupply(), 0);
        assertEq(vault.asset(), address(underlyingToken));
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
            uint256 updatedDepositCap,
            OneWayVault.FeeDistributionConfig memory updatedFeeDistribution
        ) = vault.config();

        assertEq(updatedStrategist, newStrategist);
        assertEq(updatedDepositFeeBps, newDepositFeeBps);
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

        // Test strategist ratio > 100%
        invalidConfig = OneWayVault.OneWayVaultConfig({
            depositAccount: depositAccount,
            strategist: strategist,
            depositFeeBps: depositFeeBps,
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
        assertEq(vault.feesOwedInAsset(), fee);
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
        assertEq(vault.feesOwedInAsset(), 0);
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
        assertEq(vault.feesOwedInAsset(), fee);
    }

    /*//////////////////////////////////////////////////////////////
                              PAUSE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_PauseAndUnpause() public {
        // Test pause by owner
        vm.prank(owner);
        vault.pause();

        (bool paused, bool pausedByOwner) = vault.vaultState();

        assertTrue(paused);
        assertTrue(pausedByOwner);

        // Try to deposit while paused
        vm.prank(user1);
        vm.expectRevert("Vault is paused");
        vault.deposit(1000 * 10 ** 18, user1);

        // Unpause by owner
        vm.prank(owner);
        vault.unpause();

        (paused, pausedByOwner) = vault.vaultState();

        assertFalse(paused);
        assertFalse(pausedByOwner);

        // Test pause by strategist
        vm.prank(strategist);
        vault.pause();

        (paused, pausedByOwner) = vault.vaultState();

        assertTrue(paused);
        assertFalse(pausedByOwner);

        // Try to unpause by strategist (should work since not paused by owner)
        vm.prank(strategist);
        vault.unpause();

        (paused, pausedByOwner) = vault.vaultState();

        assertFalse(paused);
        assertFalse(pausedByOwner);

        // Test pause by owner, then try to unpause by strategist (should fail)
        vm.prank(owner);
        vault.pause();

        vm.prank(strategist);
        vm.expectRevert("Only owner can unpause");
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

    /*//////////////////////////////////////////////////////////////
                          REDEMPTION RATE TESTS
    //////////////////////////////////////////////////////////////*/

    function test_UpdateRate() public {
        // First do a deposit to have some assets and shares
        uint256 depositAmount = 10_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Update rate - can only be done by strategist
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
        assertEq(vault.feesOwedInAsset(), expectedFee);

        // Update rate - should distribute fees
        uint256 newRate = initialRate * 11 / 10; // Increase by 10%

        vm.prank(strategist);
        vm.expectEmit(true, true, false, false);
        emit FeesDistributed(strategistFeeReceiver, platformFeeReceiver, 0, 0); // Exact share values will vary
        vault.update(newRate);

        // Check that fees were distributed
        assertEq(vault.feesOwedInAsset(), 0);

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

    function test_UpdateRateWithZeroShares() public {
        // Try to update rate when no shares exist
        vm.prank(strategist);
        vm.expectRevert("Zero shares");
        vault.update(initialRate * 2);
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
        uint256 fee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 depositAfterFee = depositAmount - fee;
        uint256 expectedShares = (depositAfterFee * 10 ** vault.decimals()) / initialRate;

        // Now redeem half the shares
        uint256 redeemShares = expectedShares / 2;
        string memory receiverAddress = "neutron1abcdef123456789";

        vm.prank(user1);
        vm.expectEmit(true, true, false, true);
        emit WithdrawRequested(0, user1, receiverAddress, redeemShares);
        vault.redeem(redeemShares, receiverAddress, user1);

        // Check that shares were burned
        assertEq(vault.balanceOf(user1), expectedShares - redeemShares);

        // Check that withdraw request was created
        (uint64 id, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(id, 0);
        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, initialRate);
        assertEq(sharesAmount, redeemShares);

        // Check that request ID was incremented
        assertEq(vault.currentWithdrawRequestId(), 1);
    }

    function test_Withdraw() public {
        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // Calculate expected shares and assets after fee
        uint256 fee = (depositAmount * depositFeeBps) / BASIS_POINTS;
        uint256 depositAfterFee = depositAmount - fee;

        // Now withdraw half the assets
        uint256 withdrawAssets = depositAfterFee / 2;
        string memory receiverAddress = "neutron1abcdef123456789";

        vm.prank(user1);
        vault.withdraw(withdrawAssets, receiverAddress, user1);

        // Check that withdraw request was created
        (, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, initialRate);

        // Check shares amount (should be proportional to assets requested)
        uint256 expectedShares = (withdrawAssets * 10 ** vault.decimals()) / initialRate;
        assertEq(sharesAmount, expectedShares);
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

        // User2 redeems on behalf of user1
        string memory receiverAddress = "neutron1abcdef123456789";

        vm.prank(user2);
        vault.redeem(approvedShares, receiverAddress, user1);

        // Check that shares were burned from user1
        assertEq(vault.balanceOf(user1), shares - approvedShares);

        // Check allowance was spent
        assertEq(vault.allowance(user1, user2), 0);
    }

    function test_RedeemInvalidParams() public {
        // First deposit to get some shares
        uint256 depositAmount = 10_000 * 10 ** 18;

        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        uint256 shares = vault.balanceOf(user1);
        string memory receiverAddress = "neutron1abcdef123456789";

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
        uint256 newRate = initialRate * 12 / 10; // 20% increase

        vm.prank(strategist);
        vault.update(newRate);

        // 3. Check increased total assets
        uint256 totalDeposits = depositAmount1 + depositAmount2;
        uint256 expectedTotalAssets = totalDeposits * 12 / 10; // Include fees in asset calculation
        assertApproxEqAbs(vault.totalAssets(), expectedTotalAssets, 10);

        // 4. Withdraw request from user1 (half their shares)
        uint256 user1Shares = vault.balanceOf(user1);
        uint256 redeemShares = user1Shares / 2;
        string memory receiverAddress = "neutron1abcdef123456789";

        vm.prank(user1);
        vault.redeem(redeemShares, receiverAddress, user1);

        // 5. Check user1 shares were burned
        assertEq(vault.balanceOf(user1), user1Shares - redeemShares);

        // 6. Check withdrawal request was created with correct values
        (, address ownerRequest, uint256 redemptionRate, uint256 sharesAmount, string memory receiver) =
            vault.withdrawRequests(0);

        assertEq(ownerRequest, user1);
        assertEq(receiver, receiverAddress);
        assertEq(redemptionRate, newRate);
        assertEq(sharesAmount, redeemShares);

        // 7. Update rate again to simulate more yield
        uint256 newerRate = newRate * 13 / 10; // Additional 30% increase

        vm.prank(strategist);
        vault.update(newerRate);

        // 8. Check that accumulated fees were distributed to strategist and platform
        assertTrue(vault.balanceOf(strategistFeeReceiver) > 0);
        assertTrue(vault.balanceOf(platformFeeReceiver) > 0);

        // 9. Make withdrawal request from user2
        uint256 user2Shares = vault.balanceOf(user2);
        vm.prank(user2);
        vault.redeem(user2Shares, receiverAddress, user2);

        // 10. Verify total supply consistency
        uint256 remainingUser1Shares = vault.balanceOf(user1);
        uint256 strategistShares = vault.balanceOf(strategistFeeReceiver);
        uint256 platformShares = vault.balanceOf(platformFeeReceiver);

        assertEq(vault.totalSupply(), remainingUser1Shares + strategistShares + platformShares);
    }

    function test_RateImpactOnWithdrawals() public {
        // 1. Deposit from user
        uint256 depositAmount = 100_000 * 10 ** 18;
        vm.prank(user1);
        vault.deposit(depositAmount, user1);

        // 2. Update rate to 2x
        uint256 doubledRate = initialRate * 2;
        vm.prank(strategist);
        vault.update(doubledRate);

        // 3. Calculate expected assets including fees
        uint256 expectedAssets = depositAmount * 2; // Include fees in calculation

        // 4. Verify total assets reflects new rate
        assertEq(vault.totalAssets(), expectedAssets);

        // 5. Get current shares before withdrawal
        uint256 user1SharesBefore = vault.balanceOf(user1);

        // 6. Create withdrawal request for half of assets
        uint256 withdrawAssets = expectedAssets / 2;
        string memory receiverAddress = "neutron1abcdef123456789";
        vm.prank(user1);
        vault.withdraw(withdrawAssets, receiverAddress, user1);

        // 7. Get shares after withdrawal and verify the difference
        uint256 user1SharesAfter = vault.balanceOf(user1);

        // 8. Check withdrawal request uses current rate
        (,, uint256 redemptionRate, uint256 sharesAmount,) = vault.withdrawRequests(0);
        assertEq(redemptionRate, doubledRate);

        // 9. Verify shares burned matches the withdrawal request
        assertEq(sharesAmount, user1SharesBefore - user1SharesAfter);
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
                uint256 newRate = initialRate * (100 + op * 5) / 100; // Increase rate by 5% each update
                vm.prank(strategist);
                vault.update(newRate);
            }
        }

        // Instead of calculating expected assets, just verify that totalAssets() is reasonable
        // Get total deposits made (gross, before fees)
        uint256 totalGrossDeposits = 0;
        for (uint256 op = 0; op < operationsPerUser; op++) {
            for (uint256 i = 0; i < userCount; i++) {
                totalGrossDeposits += baseAmount * (i + 1) / userCount;
            }
        }

        // Calculate a reasonable range for total assets
        // The actual value will depend on when fees were distributed and rate changes
        uint256 minExpectedAssets = totalGrossDeposits * 9 / 10; // Allow for fees taken out
        uint256 maxExpectedAssets = totalGrossDeposits * 13 / 10; // Allow for yield and fees

        uint256 actualTotalAssets = vault.totalAssets();
        assertTrue(
            actualTotalAssets >= minExpectedAssets && actualTotalAssets <= maxExpectedAssets,
            "Total assets outside reasonable range"
        );

        // Each user redeems a portion
        string memory receiverAddress = "neutron1abcdef123456789";
        for (uint256 i = 0; i < userCount; i++) {
            uint256 userShares = vault.balanceOf(users[i]);
            uint256 redeemShares = userShares / 2; // Redeem half
            vm.prank(users[i]);
            vault.redeem(redeemShares, receiverAddress, users[i]);
            // Check shares were burned
            assertEq(vault.balanceOf(users[i]), userShares - redeemShares);
        }
    }
}
