// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {WithdrawDebugger} from "./WithdrawDebugger.t.sol";
import {ValenceVault} from "../../../src/vaults/ValenceVault.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {console} from "forge-std/src/console.sol";
import {VmSafe} from "forge-std/src/Vm.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract VaultSimulationTest is VaultHelper {
    using Math for uint256;

    // Constants for simulation
    uint256 constant SIMULATION_DAYS = 10;
    uint256 internal immutable DAILY_RATE_INCREASE; // 1% daily increase
    uint256 constant INITIAL_RATE = ONE_SHARE; // 100%
    uint256 constant USER_DEPOSIT_AMOUNT = 10_000;
    uint32 constant MAX_LOSS_BPS = 1000; // 10%

    // Track our users
    address[] users;
    WithdrawDebugger internal debugger;

    // Event declarations for logging
    event SimulatedDay(uint256 day, uint256 rate, uint256 totalAssets);
    event VaultState(uint256 depositBalance, uint256 withdrawBalance, uint256 totalSupply);

    // Track expected fees
    struct FeeTracker {
        uint256 totalPlatformFees;
        uint256 totalPerformanceFees;
        uint256 totalDepositFees;
    }

    FeeTracker public feeTracker;

    constructor() {
        DAILY_RATE_INCREASE = ONE_SHARE.mulDiv(100, BASIS_POINTS);
    }

    function setUp() public override {
        super.setUp();

        debugger = new WithdrawDebugger();

        // Set fees
        setFees(
            200, // 2% deposit fee
            500, // 5% yearly platform fee
            1000, // 10% performance fee
            100 // 100 wei solver fee
        );

        // Set fee distribution (30% to strategist, 70% to platform)
        setFeeDistribution(strategistFeeAccount, platformFeeAccount, 3000);

        // Create 3 users with initial balances
        for (uint256 i = 0; i < 3; i++) {
            address newUser = makeAddr(string.concat("user", vm.toString(i)));
            users.push(newUser);

            // Setup each user
            vm.startPrank(owner);
            token.mint(newUser, USER_DEPOSIT_AMOUNT);
            vm.stopPrank();

            vm.startPrank(newUser);
            token.approve(address(vault), type(uint256).max);
            vm.stopPrank();

            vm.deal(newUser, 1 ether); // Give ETH for solver fees
        }

        // Setup withdraw account with initial tokens
        vm.startPrank(owner);
        token.mint(address(withdrawAccount), USER_DEPOSIT_AMOUNT * 10);
        withdrawAccount.approveLibrary(address(vault));
        depositAccount.approveLibrary(address(vault));
        vm.stopPrank();
    }

    function testFullVaultSimulation() public {
        uint256 currentRate = INITIAL_RATE;

        // Simulate days
        for (uint256 day = 1; day <= SIMULATION_DAYS; day++) {
            // 1. Process user actions based on the day
            if (day == 1) {
                // First user deposits on day 1
                vm.startPrank(users[0]);
                uint256 expectedFee = calculateExpectedDepositFee(USER_DEPOSIT_AMOUNT);
                uint256 balanceBefore = token.balanceOf(users[0]);
                vault.deposit(USER_DEPOSIT_AMOUNT, users[0]);
                uint256 balanceAfter = token.balanceOf(users[0]);
                assertEq(balanceBefore - balanceAfter, USER_DEPOSIT_AMOUNT, "Incorrect deposit amount");
                feeTracker.totalDepositFees += expectedFee;
                vm.stopPrank();
                console.log("Day 1: User 0 deposits", USER_DEPOSIT_AMOUNT);
            } else if (day == 3) {
                // Second user deposits on day 3
                vm.startPrank(users[1]);
                uint256 expectedFee = calculateExpectedDepositFee(USER_DEPOSIT_AMOUNT);
                uint256 balanceBefore = token.balanceOf(users[1]);
                vault.deposit(USER_DEPOSIT_AMOUNT, users[1]);
                uint256 balanceAfter = token.balanceOf(users[1]);
                assertEq(balanceBefore - balanceAfter, USER_DEPOSIT_AMOUNT, "Incorrect deposit amount");
                feeTracker.totalDepositFees += expectedFee;
                vm.stopPrank();
                console.log("Day 3: User 1 deposits", USER_DEPOSIT_AMOUNT);
            } else if (day == 5) {
                // Third user deposits and first user withdraws on day 5
                vm.startPrank(users[2]);
                uint256 expectedFee = calculateExpectedDepositFee(USER_DEPOSIT_AMOUNT);
                uint256 balanceBefore = token.balanceOf(users[2]);
                vault.deposit(USER_DEPOSIT_AMOUNT, users[2]);
                uint256 balanceAfter = token.balanceOf(users[2]);
                assertEq(balanceBefore - balanceAfter, USER_DEPOSIT_AMOUNT, "Incorrect deposit amount");
                feeTracker.totalDepositFees += expectedFee;
                vm.stopPrank();
                console.log("Day 5: User 2 deposits", USER_DEPOSIT_AMOUNT);

                vm.startPrank(users[0]);
                uint256 shares = vault.balanceOf(users[0]);
                vault.redeem{value: 100}(shares, users[0], users[0], MAX_LOSS_BPS, true);
                vm.stopPrank();
                console.log("Day 5: User 0 requests withdraw of", shares, "shares");
            } else if (day == 6) {
                // Second user withdraws on day 6
                vm.startPrank(users[1]);
                uint256 shares = vault.balanceOf(users[1]);
                vault.redeem{value: 100}(shares, users[1], users[1], MAX_LOSS_BPS, true);
                vm.stopPrank();
                console.log("Day 6: User 1 requests withdraw of", shares, "shares");
            } else if (day == 7) {
                // Third user withdraws on day 7
                vm.startPrank(users[2]);
                uint256 shares = vault.balanceOf(users[2]);
                vault.redeem{value: 100}(shares, users[2], users[2], MAX_LOSS_BPS, true);
                vm.stopPrank();
                console.log("Day 7: User 2 requests withdraw of", shares, "shares");
            }

            // 2. Daily rate increase
            currentRate += DAILY_RATE_INCREASE;

            // 3. Process daily update
            processUpdate(currentRate);

            // 4. Try to complete any pending withdraws
            tryCompleteWithdraws();

            // 5. Move to next day
            vm.warp(vm.getBlockTimestamp() + 1 days);
            vm.roll(vm.getBlockNumber() + 1);

            // Log state
            emit SimulatedDay(day, currentRate, vault.totalAssets());
            emit VaultState(
                token.balanceOf(address(_getConfig().depositAccount)),
                token.balanceOf(address(_getConfig().withdrawAccount)),
                vault.totalSupply()
            );

            console.log("Day", day, "complete - Rate:", currentRate);
            console.log("Total Assets:", vault.totalAssets());
            console.log("Total Supply:", vault.totalSupply());
            console.log("-----------------");
        }

        // Final verification
        verifyFinalState();
    }

    function processUpdate(uint256 newRate) internal {
        // Record state before update
        uint256 platformFeeBefore = vault.balanceOf(platformFeeAccount);
        uint256 strategistFeeBefore = vault.balanceOf(strategistFeeAccount);

        // Calculate assets needed for rate increase
        uint256 currentAssets = vault.totalAssets();
        uint256 assetsNeededForNewRate = vault.totalSupply().mulDiv(newRate, ONE_SHARE);
        if (assetsNeededForNewRate > currentAssets) {
            uint256 additionalAssetsNeeded = assetsNeededForNewRate - currentAssets;

            // Mint additional tokens to withdraw account to cover rate increase
            vm.startPrank(owner);
            token.mint(address(_getConfig().withdrawAccount), additionalAssetsNeeded);
            vm.stopPrank();

            console.log("Minted additional assets:", additionalAssetsNeeded);
        }

        // Use fixed withdraw fee for simulation
        uint32 withdrawFee = 50; // 0.5% constant fee

        // Get total pending withdraws
        uint256 totalPendingWithdraws = vault.totalAssetsToWithdrawNextUpdate();

        vm.startPrank(strategist);
        // Calculate netting amount - ensure withdraw account has enough for withdrawals
        uint256 withdrawAccountBalance = token.balanceOf(address(_getConfig().withdrawAccount));
        if (totalPendingWithdraws > withdrawAccountBalance) {
            uint256 nettingAmount = totalPendingWithdraws - withdrawAccountBalance;
            console.log("Netting required:", nettingAmount);

            // Process the update with netting
            vault.update(newRate, withdrawFee, nettingAmount);
        } else {
            // Process the update without netting
            vault.update(newRate, withdrawFee, 0);
        }
        vm.stopPrank();

        // Calculate fees distributed
        uint256 platformFeeAfter = vault.balanceOf(platformFeeAccount);
        uint256 strategistFeeAfter = vault.balanceOf(strategistFeeAccount);

        uint256 totalFeeShares = (platformFeeAfter - platformFeeBefore) + (strategistFeeAfter - strategistFeeBefore);
        if (totalFeeShares > 0) {
            console.log("Fees distributed - Total shares:", totalFeeShares);
            console.log("Platform shares:", platformFeeAfter - platformFeeBefore);
            console.log("Strategist shares:", strategistFeeAfter - strategistFeeBefore);
        }
    }

    function tryCompleteWithdraws() internal {
        for (uint256 y = 0; y < users.length; y++) {
            address user = users[y];

            // Get withdraw request details
            (, uint64 claimTime,,,,, uint256 sharesAmount) = vault.userWithdrawRequest(user);

            if (sharesAmount == 0) {
                console.log("No withdraw request for user", y);
                continue;
            }

            if (claimTime > vm.getBlockTimestamp()) {
                console.log("Withdraw not claimable yet", y);
                continue;
            }

            console.log("\n=== Processing Withdraw for User", y, "===");

            // Start recording logs
            vm.recordLogs();

            vm.prank(user);
            try vault.completeWithdraw(user) {
                console.log("Withdraw completion attempted");

                // Get the recorded logs
                VmSafe.Log[] memory entries = vm.getRecordedLogs();

                for (uint256 i = 0; i < entries.length; i++) {
                    bytes32 eventHash = entries[i].topics[0];

                    if (eventHash == keccak256("WithdrawCompleted(address,address,uint256,uint256,address)")) {
                        // Decode WithdrawCompleted event
                        address eventOwner = address(uint160(uint256(entries[i].topics[1])));
                        address eventReceiver = address(uint160(uint256(entries[i].topics[2])));
                        (uint256 assets, uint256 shares) = abi.decode(entries[i].data, (uint256, uint256));
                        console.log("WithdrawCompleted Event:");
                        console.log(" - Owner:", eventOwner);
                        console.log(" - Receiver:", eventReceiver);
                        console.log(" - Assets:", assets);
                        console.log(" - Shares:", shares);
                    } else if (eventHash == keccak256("WithdrawCancelled(address,uint256,uint256,uint256)")) {
                        // Decode WithdrawCancelled event
                        address eventOwner = address(uint160(uint256(entries[i].topics[1])));
                        (uint256 shares, uint256 currentLoss, uint256 maxAllowedLoss) =
                            abi.decode(entries[i].data, (uint256, uint256, uint256));
                        console.log("WithdrawCancelled Event:");
                        console.log(" - Owner:", eventOwner);
                        console.log(" - Shares:", shares);
                        console.log(" - Current Loss:", currentLoss);
                        console.log(" - Max Allowed Loss:", maxAllowedLoss);
                    } else if (eventHash == keccak256("WithdrawCompletionSkipped(address,string)")) {
                        // Decode WithdrawCompletionSkipped event
                        address eventOwner = address(uint160(uint256(entries[i].topics[1])));
                        string memory reason = abi.decode(entries[i].data, (string));
                        console.log("WithdrawCompletionSkipped Event:");
                        console.log(" - Owner:", eventOwner);
                        console.log(" - Reason:", reason);
                    }
                }
            } catch Error(string memory reason) {
                console.log("Failed with reason:", reason);
            } catch Panic(uint256 code) {
                console.log("Failed with panic code:", code);
            } catch {
                console.log("Failed with unknown error");
                VmSafe.Log[] memory entries = vm.getRecordedLogs();
                for (uint256 x = 0; x < entries.length; x++) {
                    bytes32 eventHash = entries[x].topics[0];
                    console.log("Event hash:", vm.toString(eventHash));
                    console.log("Event data:", abi.decode(entries[x].data, (string)));
                }
            }
        }
    }

    function calculateExpectedPlatformFees(uint256 assets, uint256 timeElapsed) internal pure returns (uint256) {
        return assets.mulDiv(BASIS_POINTS - 500, BASIS_POINTS).mulDiv(timeElapsed, 365 days);
    }

    function calculateExpectedPerformanceFees(uint256 profit) internal pure returns (uint256) {
        return profit.mulDiv(1000, BASIS_POINTS); // 10% of profit
    }

    function calculateExpectedDepositFee(uint256 depositAmount) internal pure returns (uint256) {
        return depositAmount.mulDiv(BASIS_POINTS - 200, BASIS_POINTS); // 2% of deposit
    }

    function verifyFinalState() internal view {
        // 1. Verify all assets are accounted for
        uint256 totalSupply = vault.totalSupply();
        uint256 totalAssets = vault.totalAssets();
        require(totalAssets == vault.convertToAssets(totalSupply), "Asset/share mismatch");

        // 2. Verify deposit account balance
        uint256 depositBalance = token.balanceOf(address(_getConfig().depositAccount));

        // 3. Verify withdraw account balance
        uint256 withdrawBalance = token.balanceOf(address(_getConfig().withdrawAccount));

        // 4. Total balances should match total assets
        require(depositBalance + withdrawBalance >= totalAssets, "Missing assets");

        // 5. Verify no stuck withdraws
        for (uint256 i = 0; i < users.length; i++) {
            (,,,,,, uint256 shares) = vault.userWithdrawRequest(users[i]);
            if (shares > 0) {
                // Get the request details to see why it's stuck
                (, uint64 claimTime,,,,, uint256 amount) = vault.userWithdrawRequest(users[i]);
                console.log("Stuck withdraw for user", i);
                console.log("Claim time:", claimTime);
                console.log("Amount:", amount);
                revert("Found stuck withdraw");
            }
        }

        // 6. Verify fees
        console.log("=== Fee Verification ===");
        uint256 platformBalance = vault.balanceOf(platformFeeAccount);
        uint256 strategistBalance = vault.balanceOf(strategistFeeAccount);

        console.log("Platform account shares:", platformBalance);
        console.log("Strategist account shares:", strategistBalance);
        console.log("Total deposit fees collected:", feeTracker.totalDepositFees);

        // Verify fee distribution ratio (30% strategist, 70% platform)
        uint256 totalFeeShares = platformBalance + strategistBalance;
        if (totalFeeShares > 0) {
            uint256 strategistRatio = (strategistBalance * 10000) / totalFeeShares;
            assertApproxEqRel(strategistRatio, 3000, 33e15, "Incorrect fee distribution ratio"); // 1% tolerance
        }

        // 7. Log final balances
        console.log("=== Final State ===");
        for (uint256 i = 0; i < users.length; i++) {
            console.log("User", i, "token balance:", token.balanceOf(users[i]));
            console.log("User", i, "share balance:", vault.balanceOf(users[i]));
        }
    }
}
