// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../../src/vaults/ValenceVault.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {console} from "forge-std/src/console.sol";

contract VaultGasBenchmarkTest is VaultHelper {
    // Constants for benchmarking
    uint256 constant DEPOSIT_AMOUNT = 1000;
    uint256 constant WITHDRAW_AMOUNT = 500;
    uint32 constant MAX_LOSS = 500; // 5%
    uint256[] BATCH_SIZES = [1, 2, 5, 10, 20, 50];

    // Track addresses for batch operations
    address[] users;
    address solver;

    function setUp() public override {
        super.setUp();
        solver = makeAddr("solver");
        vm.deal(solver, 100 ether); // Give solver enough ETH for fees
    }

    // Helper to setup users with deposits for batch testing
    function setupUsersForBatchTesting(uint256 numUsers) internal {
        for (uint256 i = 0; i < numUsers; i++) {
            address newUser = makeAddr(string.concat("user", vm.toString(i)));
            users.push(newUser);

            // Setup tokens and approvals
            vm.startPrank(owner);
            token.mint(newUser, INITIAL_USER_BALANCE);
            vm.stopPrank();

            vm.startPrank(newUser);
            token.approve(address(vault), type(uint256).max);
            vault.deposit(DEPOSIT_AMOUNT, newUser);
            vm.deal(newUser, 1 ether); // For solver fees
            vm.stopPrank();
        }

        // Setup withdraw account with enough tokens
        vm.startPrank(owner);
        token.mint(address(withdrawAccount), INITIAL_USER_BALANCE);
        vm.stopPrank();
    }

    // Test gas for basic operations sequence
    function testGasBasicOperationsSequence() public {
        // Setup
        vm.startPrank(user);

        // Measure deposit gas
        uint256 gasStart = gasleft();
        vault.deposit(DEPOSIT_AMOUNT, user);
        uint256 depositGas = gasStart - gasleft();

        // Measure withdraw request gas
        gasStart = gasleft();
        vault.withdraw(WITHDRAW_AMOUNT, user, user, MAX_LOSS, false);
        uint256 withdrawRequestGas = gasStart - gasleft();
        vm.stopPrank();

        // Measure update gas
        vm.startPrank(strategist);
        gasStart = gasleft();
        _update(BASIS_POINTS, 100, WITHDRAW_AMOUNT);
        uint256 updateGas = gasStart - gasleft();
        vm.stopPrank();

        // Fast forward and measure withdraw completion gas
        vm.warp(vm.getBlockTimestamp() + 4 days);
        vm.startPrank(user);
        gasStart = gasleft();
        vault.completeWithdraw(user);
        uint256 completeWithdrawGas = gasStart - gasleft();
        vm.stopPrank();

        // Log results
        console.log("Gas Costs for Basic Operations:");
        console.log("Deposit:", depositGas);
        console.log("Withdraw Request:", withdrawRequestGas);
        console.log("Update:", updateGas);
        console.log("Complete Withdraw:", completeWithdrawGas);
        console.log("Total Gas:", depositGas + withdrawRequestGas + updateGas + completeWithdrawGas);
    }

    // Test gas costs for batch operations with different sizes
    function testGasBatchOperationsWithDifferentSizes() public {
        // Setup solver fee
        setFees(0, 0, 0, 100);

        // Test each batch size
        for (uint256 i = 0; i < BATCH_SIZES.length; i++) {
            uint256 batchSize = BATCH_SIZES[i];
            console.log("\nTesting batch size:", batchSize);

            // Reset state and setup users
            _resetTestState();
            setupUsersForBatchTesting(batchSize);

            // Setup withdraw requests for all users
            for (uint256 j = 0; j < batchSize; j++) {
                vm.prank(users[j]);
                vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[j], users[j], MAX_LOSS, true);
            }

            // Process update
            vm.startPrank(strategist);
            _update(BASIS_POINTS, 100, WITHDRAW_AMOUNT * batchSize);
            vm.stopPrank();

            // Fast forward time
            vm.warp(vm.getBlockTimestamp() + 4 days);

            // Measure gas for batch completion
            vm.startPrank(solver);
            uint256 gasStart = gasleft();
            vault.completeWithdraws(users);
            uint256 batchGas = gasStart - gasleft();
            vm.stopPrank();

            // Calculate and log metrics
            uint256 gasPerOperation = batchGas / batchSize;
            console.log("Total Gas Used:", batchGas);
            console.log("Gas Per Operation:", gasPerOperation);
        }
    }

    // Test gas costs for mixed operations
    function testGasMixedOperations() public {
        // Setup initial state with 10 users
        setupUsersForBatchTesting(10);
        setFees(0, 0, 0, 100);

        // Track gas usage for different operation combinations
        uint256 gasStart;

        // 1. Sequential deposits
        gasStart = gasleft();
        for (uint256 i = 0; i < 5; i++) {
            vm.prank(users[i]);
            vault.deposit(DEPOSIT_AMOUNT, users[i]);
        }
        uint256 sequentialDepositsGas = gasStart - gasleft();

        // 2. Mixed deposits and withdraws
        gasStart = gasleft();
        for (uint256 i = 0; i < 5; i++) {
            // Alternate between deposit and withdraw
            if (i % 2 == 0) {
                vm.prank(users[i]);
                vault.deposit(DEPOSIT_AMOUNT, users[i]);
            } else {
                vm.prank(users[i]);
                vault.withdraw{value: 100}(WITHDRAW_AMOUNT, users[i], users[i], MAX_LOSS, true);
            }
        }
        uint256 mixedOperationsGas = gasStart - gasleft();

        // Process update
        vm.startPrank(strategist);
        _update(BASIS_POINTS, 100, WITHDRAW_AMOUNT * 3);
        vm.stopPrank();

        // Fast forward time
        vm.warp(vm.getBlockTimestamp() + 4 days);

        // Log results
        console.log("\nGas Costs for Mixed Operations:");
        console.log("Sequential Deposits:", sequentialDepositsGas);
        console.log("Mixed Deposits/Withdraws:", mixedOperationsGas);
        console.log(
            "Gas Difference:",
            mixedOperationsGas > sequentialDepositsGas
                ? mixedOperationsGas - sequentialDepositsGas
                : sequentialDepositsGas - mixedOperationsGas
        );
    }

    // Helper to reset test state between batch tests
    function _resetTestState() internal {
        // Clear users array
        delete users;
    }
}
