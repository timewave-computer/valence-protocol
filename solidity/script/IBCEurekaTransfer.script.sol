// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {MockERC20} from "../test/mocks/MockERC20.sol";
import {IBCEurekaTransfer} from "../src/libraries/IBCEurekaTransfer.sol";
import {IEurekaHandler} from "../src/libraries/interfaces/eureka/IEurekaHandler.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract IBCEurekaTransferScript is Script {
    // Address of the WETH ERC-20 token on Ethereum
    address constant WETH_ADDR = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

    // Address of the Eureka Handler on Ethereum
    address constant EUREKA_HANDLER = 0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C;

    // Example addresses for owner, processor, and fee recipient
    address owner = address(1);
    address processor = address(2);
    address feeRecipient = address(3);

    // Contracts
    IBCEurekaTransfer public ibcEurekaTransfer;
    IBCEurekaTransfer public ibcEurekaTransferFull;
    BaseAccount inputAccount;

    // Amount to transfer
    uint256 tokenAmount = 100000; // For WETH
    uint64 timeoutTimestamp = 3600; // 1 hour timeout

    // Cosmos recipient address in bech32 format
    string recipient = "cosmos14mlpd48k5vkeset4x7f78myz3m47jcax4mesvx";
    string sourceClient = "cosmoshub-0";

    function run() external {
        // Create a fork of Ethereum mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Replace the runtime code at WETH_ADDR with our MockERC20 code so we can mint some WETH
        bytes memory mockCode = type(MockERC20).runtimeCode;
        vm.etch(WETH_ADDR, mockCode);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        // Mint some WETH tokens to the BaseAccount
        MockERC20 weth = MockERC20(WETH_ADDR);
        weth.mint(address(inputAccount), tokenAmount * 2);

        // Deploy a new IBCEurekaTransfer contract with fixed amount
        IBCEurekaTransfer.IBCEurekaTransferConfig memory wethConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: tokenAmount,
            transferToken: WETH_ADDR,
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeoutTimestamp,
            eurekaHandler: IEurekaHandler(EUREKA_HANDLER)
        });
        bytes memory wethConfigBytes = abi.encode(wethConfig);
        ibcEurekaTransfer = new IBCEurekaTransfer(owner, processor, wethConfigBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(ibcEurekaTransfer));

        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 wethBalanceBefore = weth.balanceOf(address(inputAccount));
        console.log("WETH balance before transfer: ", wethBalanceBefore);

        // Create a fee structure for the transfer
        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 1000, // 1000 units of WETH as relay fee
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 300) // Quote expires in 5 minutes
        });

        // Execute the WETH transfer
        vm.prank(processor);
        ibcEurekaTransfer.transfer(fees, "");

        // Get the balance after the transfer
        uint256 wethBalanceAfter = weth.balanceOf(address(inputAccount));
        console.log("WETH balance after transfer: ", wethBalanceAfter);

        // Check balance changes
        console.log("WETH sent: ", wethBalanceBefore - wethBalanceAfter);

        // Verify that the balances have been correctly deducted (tokenAmount)
        assert(wethBalanceBefore - wethBalanceAfter == tokenAmount);

        // Check that fee recipient received the fees
        uint256 feeRecipientBalance = weth.balanceOf(feeRecipient);
        console.log("Fee recipient balance: ", feeRecipientBalance);
        assert(feeRecipientBalance == 1000); // The relay fee amount

        // Deploy a new IBCEurekaTransfer contract for full balance transfer
        vm.startPrank(owner);
        IBCEurekaTransfer.IBCEurekaTransferConfig memory fullBalanceConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 0, // 0 means transfer full balance
            transferToken: WETH_ADDR,
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeoutTimestamp,
            eurekaHandler: IEurekaHandler(EUREKA_HANDLER)
        });
        bytes memory fullBalanceConfigBytes = abi.encode(fullBalanceConfig);
        ibcEurekaTransferFull = new IBCEurekaTransfer(owner, processor, fullBalanceConfigBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(ibcEurekaTransferFull));
        vm.stopPrank();

        uint256 wethBalanceBeforeFullTransfer = weth.balanceOf(address(inputAccount));
        console.log("WETH balance before full transfer: ", wethBalanceBeforeFullTransfer);

        // Create a fee structure for the full transfer
        IEurekaHandler.Fees memory fullTransferFees = IEurekaHandler.Fees({
            relayFee: 1000, // 1000 units of WETH as relay fee
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 300) // Quote expires in 5 minutes
        });

        // Execute the WETH transfer for full amount
        vm.prank(processor);
        ibcEurekaTransferFull.transfer(fullTransferFees, "test memo");

        uint256 wethBalanceAfterFullTransfer = weth.balanceOf(address(inputAccount));
        console.log("WETH balance after full transfer: ", wethBalanceAfterFullTransfer);

        // Check that all WETH has been transferred
        assert(wethBalanceAfterFullTransfer == 0);

        // Check total fees received by fee recipient
        uint256 finalFeeRecipientBalance = weth.balanceOf(feeRecipient);
        console.log("Final fee recipient balance: ", finalFeeRecipientBalance);
        assert(finalFeeRecipientBalance == 2000); // Two relay fees of 1000 each
    }
}
