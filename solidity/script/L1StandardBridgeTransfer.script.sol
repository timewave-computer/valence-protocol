// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {StandardBridgeTransfer} from "../src/libraries/StandardBridgeTransfer.sol";
import {IStandardBridge} from "../src/libraries/interfaces/standard-bridge/IStandardBridge.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract L1BaseBridgeTransferScript is Script {
    // Address of the USDC ERC-20 token on Ethereum
    address constant USDC_L1_ADDR = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;

    // Address of the corresponding USDC on Base
    address constant USDC_L2_ADDR = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913; // USDC on Base

    // Address of the L1StandardBridge for Base
    address constant L1_BASE_BRIDGE = 0x3154Cf16ccdb4C6d922629664174b904d80F2C35;

    // Address of USDC Whale
    address constant USDC_WHALE = 0x28C6c06298d514Db089934071355E5743bf21d60;

    // Example addresses for owner, processor, and recipient
    address owner = address(1);
    address processor = address(2);
    address recipient = address(3); // Base recipient address

    // Contracts
    StandardBridgeTransfer public standardBridgeTransferNative;
    StandardBridgeTransfer public standardBridgeTransferERC20;
    StandardBridgeTransfer public standardBridgeTransferNativeFull;
    BaseAccount inputAccount;

    // Amounts to transfer
    uint256 ethAmount = 0.1 ether; // For native ETH
    uint256 tokenAmount = 100000; // For USDC
    uint32 minGasLimit = 200000; // Standard gas limit for L1->L2 messages

    function run() external {
        // Create a fork of Ethereum mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        vm.stopPrank();

        // Fund the input account with USDC
        vm.startPrank(USDC_WHALE);
        uint256 amountToFund = 1000 * 10 ** 6; // 1000 USDC
        IERC20(USDC_L1_ADDR).transfer(address(inputAccount), amountToFund);
        vm.stopPrank();

        // Send some ETH to the BaseAccount
        vm.deal(address(inputAccount), ethAmount * 3);

        vm.startPrank(owner);

        // Deploy a new StandardBridgeTransfer contract for native ETH with fixed amount
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: ethAmount,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: IStandardBridge(payable(L1_BASE_BRIDGE)),
            token: address(0), // Native ETH
            remoteToken: address(0), // Not needed for ETH
            minGasLimit: minGasLimit,
            extraData: ""
        });
        bytes memory ethConfigBytes = abi.encode(ethConfig);
        standardBridgeTransferNative = new StandardBridgeTransfer(owner, processor, ethConfigBytes);

        // Deploy a new StandardBridgeTransfer contract for USDC
        StandardBridgeTransfer.StandardBridgeTransferConfig memory usdcConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: tokenAmount,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: IStandardBridge(payable(L1_BASE_BRIDGE)),
            token: USDC_L1_ADDR,
            remoteToken: USDC_L2_ADDR, // Base USDC address
            minGasLimit: minGasLimit,
            extraData: ""
        });
        bytes memory usdcConfigBytes = abi.encode(usdcConfig);
        standardBridgeTransferERC20 = new StandardBridgeTransfer(owner, processor, usdcConfigBytes);

        // Approve the libraries from the input account
        inputAccount.approveLibrary(address(standardBridgeTransferNative));
        inputAccount.approveLibrary(address(standardBridgeTransferERC20));

        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 ethBalanceBefore = address(inputAccount).balance;
        uint256 usdcBalanceBefore = IERC20(USDC_L1_ADDR).balanceOf(address(inputAccount));

        console.log("ETH balance before transfer: ", ethBalanceBefore);
        console.log("USDC balance before transfer: ", usdcBalanceBefore);

        // Execute the native ETH transfer
        vm.prank(processor);
        standardBridgeTransferNative.transfer();

        // Execute the USDC transfer
        vm.prank(processor);
        standardBridgeTransferERC20.transfer();

        // Get the balance after the transfers
        uint256 ethBalanceAfter = address(inputAccount).balance;
        uint256 usdcBalanceAfter = IERC20(USDC_L1_ADDR).balanceOf(address(inputAccount));

        console.log("ETH balance after transfer: ", ethBalanceAfter);
        console.log("USDC balance after transfer: ", usdcBalanceAfter);

        // Check balance changes
        console.log("ETH used: ", ethBalanceBefore - ethBalanceAfter);
        console.log("USDC sent: ", usdcBalanceBefore - usdcBalanceAfter);

        // Verify that the balances have been correctly deducted
        assert(ethBalanceBefore - ethBalanceAfter == ethAmount);
        assert(usdcBalanceBefore - usdcBalanceAfter == tokenAmount);

        // Deploy a new StandardBridgeTransfer contract for native ETH with full balance transfer
        vm.startPrank(owner);
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfigFull = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 0, // 0 means transfer full balance
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: IStandardBridge(payable(L1_BASE_BRIDGE)),
            token: address(0), // Native ETH
            remoteToken: address(0), // Not needed for ETH
            minGasLimit: minGasLimit,
            extraData: ""
        });
        bytes memory ethConfigFullBytes = abi.encode(ethConfigFull);
        standardBridgeTransferNativeFull = new StandardBridgeTransfer(owner, processor, ethConfigFullBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(standardBridgeTransferNativeFull));
        vm.stopPrank();

        uint256 ethBalanceBeforeFullTransfer = address(inputAccount).balance;
        console.log("ETH balance before full transfer: ", ethBalanceBeforeFullTransfer);

        // Execute the native ETH transfer for full amount
        vm.prank(processor);
        standardBridgeTransferNativeFull.transfer();

        uint256 ethBalanceAfterFullTransfer = address(inputAccount).balance;
        console.log("ETH balance after full transfer: ", ethBalanceAfterFullTransfer);

        assert(ethBalanceAfterFullTransfer == 0); // All ETH should be transferred
    }
}
