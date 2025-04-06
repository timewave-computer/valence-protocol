// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {MockERC20} from "../test/mocks/MockERC20.sol";
import {MockStargate} from "../test/mocks/MockStargate.sol";
import {StargateTransfer} from "../src/libraries/StargateTransfer.sol";
import {IStargate} from "@stargatefinance/stg-evm-v2/src/interfaces/IStargate.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract StargateTransferScript is Script {
    // Address of the USDC ERC-20 token
    address constant USDC_ADDR = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;

    // Address of the Stargate v2 pool for USDC
    address constant STARGATE_USDC_POOL = 0xc026395860Db2d07ee33e05fE50ed7bD583189C7;
    // Address of the Stargate v2 pool for native ETH
    address constant STARGATE_ETH_POOL = 0x77b2043768d28E9C9aB44E1aBfC95944bcE57931;
    // Example addresses for owner and processor
    address owner = address(1);
    address processor = address(2);
    address recipient = address(3);

    // Contracts
    StargateTransfer public stargateTransferNative;
    StargateTransfer public stargateTransferERC20;
    StargateTransfer public stargateTransferNativeFull;
    BaseAccount inputAccount;

    // Amounts to transfer
    uint256 ethAmount = 0.1 ether; // For native ETH
    uint256 tokenAmount = 100000; // For USDC

    // Destination domain (chain ID in Stargate format)
    uint32 destinationDomain = 30362; // BeraChain

    function run() external {
        // Create a fork of mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Replace the runtime code at USDC_ADDR with our MockERC20 code so we can mint some USDC to the BaseAccount
        bytes memory mockCode = type(MockERC20).runtimeCode;
        vm.etch(USDC_ADDR, mockCode);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        // Mint some USDC tokens to the BaseAccount
        MockERC20 usdc = MockERC20(USDC_ADDR);
        usdc.mint(address(inputAccount), tokenAmount); // Mint the USDC amount we want to transfer

        // Send some ETH to the BaseAccount
        // We are going to transfer first a fixed amount and then the remaining full amount to test both scenarios
        vm.deal(address(inputAccount), ethAmount * 2);

        // Deploy a new StargateTransfer contract for native ETH (Taxi mode)
        StargateTransfer.StargateTransferConfig memory ethConfig = StargateTransfer.StargateTransferConfig({
            recipient: bytes32(uint256(uint160(recipient))),
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            stargateAddress: IStargate(STARGATE_ETH_POOL),
            transferToken: address(0), // Native ETH
            amount: ethAmount,
            minAmountToReceive: 0, // Let the contract calculate
            refundAddress: address(0), // Default refund address
            extraOptions: "", // No extra options
            composeMsg: "", // No compose message
            oftCmd: "" // Taxi mode (empty bytes)
        });
        bytes memory ethConfigBytes = abi.encode(ethConfig);
        stargateTransferNative = new StargateTransfer(owner, processor, ethConfigBytes);

        // Deploy a new StargateTransfer contract for USDC (Bus mode)
        StargateTransfer.StargateTransferConfig memory usdcConfig = StargateTransfer.StargateTransferConfig({
            recipient: bytes32(uint256(uint160(recipient))),
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            stargateAddress: IStargate(STARGATE_USDC_POOL),
            transferToken: USDC_ADDR,
            amount: tokenAmount,
            minAmountToReceive: 0, // Let the contract calculate
            refundAddress: address(0), // Default refund address
            extraOptions: "", // No extra options
            composeMsg: "", // No compose message
            oftCmd: hex"01" // Bus mode (bytes(1))
        });
        bytes memory usdcConfigBytes = abi.encode(usdcConfig);
        stargateTransferERC20 = new StargateTransfer(owner, processor, usdcConfigBytes);

        // Approve the libraries from the input account
        inputAccount.approveLibrary(address(stargateTransferNative));
        inputAccount.approveLibrary(address(stargateTransferERC20));

        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 ethBalanceBefore = address(inputAccount).balance;
        uint256 usdcBalanceBefore = usdc.balanceOf(address(inputAccount));

        console.log("ETH balance before transfer: ", ethBalanceBefore);
        console.log("USDC balance before transfer: ", usdcBalanceBefore);

        // Execute the native ETH transfer (Taxi mode)
        vm.prank(processor);
        stargateTransferNative.transfer();

        // Execute the USDC transfer (Bus mode)
        vm.prank(processor);
        stargateTransferERC20.transfer();

        // Get the balance after the transfers
        uint256 ethBalanceAfter = address(inputAccount).balance;
        uint256 usdcBalanceAfter = usdc.balanceOf(address(inputAccount));

        console.log("ETH balance after transfer: ", ethBalanceAfter);
        console.log("USDC balance after transfer: ", usdcBalanceAfter);

        // Check balance changes
        console.log("ETH used: ", ethBalanceBefore - ethBalanceAfter);
        console.log("USDC sent: ", usdcBalanceBefore - usdcBalanceAfter);

        // The ETH balance will be less than the initial amount due to fees
        assert(ethBalanceBefore > ethBalanceAfter);

        // The USDC balance should be exactly reduced by the amount
        assert(usdcBalanceBefore - usdcBalanceAfter == tokenAmount);

        // Deploy a new StargateTransfer contract for native ETH (Taxi mode) but this time for full amount
        vm.startPrank(owner);
        StargateTransfer.StargateTransferConfig memory ethConfigFull = StargateTransfer.StargateTransferConfig({
            recipient: bytes32(uint256(uint160(recipient))),
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            stargateAddress: IStargate(STARGATE_ETH_POOL),
            transferToken: address(0), // Native ETH
            amount: 0,
            minAmountToReceive: 0, // Let the contract calculate
            refundAddress: address(0), // Default refund address
            extraOptions: "", // No extra options
            composeMsg: "", // No compose message
            oftCmd: "" // Taxi mode (empty bytes)
        });
        bytes memory ethConfigFullBytes = abi.encode(ethConfigFull);
        stargateTransferNativeFull = new StargateTransfer(owner, processor, ethConfigFullBytes);

        // Approve the libraries from the input account
        inputAccount.approveLibrary(address(stargateTransferNativeFull));
        vm.stopPrank();

        uint256 ethBalanceBeforeFullTransfer = address(inputAccount).balance;
        console.log("ETH balance before transfer: ", ethBalanceBeforeFullTransfer);

        // Execute the native ETH transfer (Taxi mode) for full amount
        vm.prank(processor);
        stargateTransferNativeFull.transfer();

        uint256 ethBalanceAfterFullTransfer = address(inputAccount).balance;
        // Some dust should left in the account
        assert(ethBalanceAfterFullTransfer > 0);
        assert(ethBalanceAfterFullTransfer < 0.000001 ether);
        console.log("ETH balance after transfer: ", ethBalanceAfterFullTransfer);
    }
}
