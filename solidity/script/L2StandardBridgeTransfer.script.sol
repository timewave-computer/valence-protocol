// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {MockERC20} from "../test/mocks/MockERC20.sol";
import {StandardBridgeTransfer} from "../src/libraries/StandardBridgeTransfer.sol";
import {IStandardBridge} from "../src/libraries/interfaces/standard-bridge/IStandardBridge.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract L2StandardBridgeTransferScript is Script {
    // Address of the test USDC ERC-20 token
    address constant USDC_L2_ADDR = 0x7F5c764cBc14f9669B88837ca1490cCa17c31607; // USDC on Optimism

    // Address of the corresponding L1 USDC (for remoteToken parameter)
    address constant USDC_L1_ADDR = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48; // USDC on Ethereum

    // Address of the L2StandardBridge on Optimism
    address constant L2_STANDARD_BRIDGE = 0x4200000000000000000000000000000000000010;

    // Example addresses for owner, processor, and recipient
    address owner = address(1);
    address processor = address(2);
    address recipient = address(3); // L1 recipient address

    // Contracts
    StandardBridgeTransfer public standardBridgeTransferNative;
    StandardBridgeTransfer public standardBridgeTransferERC20;
    StandardBridgeTransfer public standardBridgeTransferNativeFull;
    BaseAccount inputAccount;

    // Amounts to transfer
    uint256 ethAmount = 0.1 ether; // For native ETH
    uint256 tokenAmount = 100000; // For USDC
    uint32 minGasLimit = 200000; // Standard gas limit for L2->L1 messages

    function run() external {
        // Create a fork of Optimism and switch to it
        uint256 forkId = vm.createFork("https://mainnet.optimism.io");
        vm.selectFork(forkId);

        // Replace the runtime code at USDC_L2_ADDR with our MockERC20 code so we can mint some USDC
        bytes memory mockCode = type(MockERC20).runtimeCode;
        vm.etch(USDC_L2_ADDR, mockCode);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        // Mint some USDC tokens to the BaseAccount
        MockERC20 usdc = MockERC20(USDC_L2_ADDR);
        usdc.mint(address(inputAccount), tokenAmount);

        // Send some ETH to the BaseAccount
        vm.deal(address(inputAccount), ethAmount * 3);

        // Deploy a new StandardBridgeTransfer contract for native ETH with fixed amount
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: ethAmount,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: IStandardBridge(payable(L2_STANDARD_BRIDGE)),
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
            standardBridge: IStandardBridge(payable(L2_STANDARD_BRIDGE)),
            token: USDC_L2_ADDR,
            remoteToken: USDC_L1_ADDR, // L1 USDC address
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
        uint256 usdcBalanceBefore = usdc.balanceOf(address(inputAccount));

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
        uint256 usdcBalanceAfter = usdc.balanceOf(address(inputAccount));

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
            standardBridge: IStandardBridge(payable(L2_STANDARD_BRIDGE)),
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
