// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IBCEurekaTransfer} from "../src/libraries/IBCEurekaTransfer.sol";
import {IEurekaHandler} from "../src/libraries/interfaces/eureka/IEurekaHandler.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract LombardIBCEurekaTransferScript is Script {
    // Address of the LBTC ERC-20 token on Ethereum
    address constant LBTC_ADDR = 0x8236a87084f8B84306f72007F36F2618A5634494;

    // Address of the Eureka Handler on Ethereum
    address constant EUREKA_HANDLER = 0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C;

    // Address of LBTC Whale
    address constant LBTC_WHALE = 0x89F2de2b541C443745E180b239A8110abB9d00f4;

    // Example addresses for owner, processor, and fee recipient
    address owner = address(1);
    address processor = address(2);
    address feeRecipient = address(3);

    // Contracts
    IBCEurekaTransfer public ibcEurekaTransfer;
    BaseAccount inputAccount;

    // Lombard recipient address in bech32 format
    string recipient = "lom14mlpd48k5vkeset4x7f78myz3m47jcaxp0gn00";
    string sourceClient = "ledger-mainnet-1";

    function run() external {
        // Create a fork of Ethereum mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        vm.stopPrank();

        // Fund the input account with WETH
        vm.startPrank(LBTC_WHALE);
        uint256 amountToFund = 1 * 10 ** 8; // 1 LBTC (has 8 decimals)
        IERC20(LBTC_ADDR).transfer(address(inputAccount), amountToFund);
        vm.stopPrank();

        uint64 timeoutTimestamp = 3600; // 1 hour timeout

        vm.startPrank(owner);
        // Deploy a new IBCEurekaTransfer contract with fixed amount
        IBCEurekaTransfer.IBCEurekaTransferConfig memory lbtcConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 0, // 0 means transfer the entire amount
            minAmountOut: 0, // Means same as amount
            transferToken: LBTC_ADDR,
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeoutTimestamp,
            eurekaHandler: IEurekaHandler(EUREKA_HANDLER)
        });
        bytes memory lbtcConfigBytes = abi.encode(lbtcConfig);
        ibcEurekaTransfer = new IBCEurekaTransfer(owner, processor, lbtcConfigBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(ibcEurekaTransfer));

        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 lbtcBalanceBefore = IERC20(LBTC_ADDR).balanceOf(address(inputAccount));
        console.log("LBTC balance before transfer: ", lbtcBalanceBefore);

        // Create a fee structure for the transfer
        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 1000, // 1000 units of LBTC as relay fee
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 300) // Quote expires in 5 minutes
        });

        // Execute the LBTC lombard transfer
        vm.prank(processor);
        ibcEurekaTransfer.lombardTransfer(fees, "");

        // Get the balance after the transfer
        uint256 lbtcBalanceAfter = IERC20(LBTC_ADDR).balanceOf(address(inputAccount));
        console.log("LBTC balance after transfer: ", lbtcBalanceAfter);

        // Verify that balance was deducted
        assert(lbtcBalanceBefore > lbtcBalanceAfter);
        assert(lbtcBalanceAfter == 0); // All LBTC should be transferred

        // Check that fee recipient received the fees
        uint256 feeRecipientBalance = IERC20(LBTC_ADDR).balanceOf(feeRecipient);
        console.log("Fee recipient balance: ", feeRecipientBalance);
        assert(feeRecipientBalance == 1000); // The relay fee amount
    }
}
