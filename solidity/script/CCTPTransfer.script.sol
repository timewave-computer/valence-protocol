// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {CCTPTransfer} from "../src/libraries/CCTPTransfer.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {ITokenMessenger} from "../src/libraries/interfaces/cctp/ITokenMessenger.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract CCTPTransferScript is Script {
    // Address of the USDC ERC-20 token
    address constant USDC_ADDR = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    // Address of the CCTP Token Messenger
    address constant CCTP_TOKEN_MESSENGER_ADDR = 0xBd3fa81B58Ba92a82136038B25aDec7066af3155;
    // Address of USDC Whale
    address constant USDC_WHALE = 0x28C6c06298d514Db089934071355E5743bf21d60;

    address owner = address(1);
    address processor = address(2);
    CCTPTransfer public cctpTransfer;
    BaseAccount inputAccount;
    uint256 amount = 100000;

    function run() external {
        // Create a fork of mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Start broadcasting transactions
        vm.startPrank(owner);
        // Deploy a new BaseAccount contract.
        inputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Fund the input account with USDC
        vm.startPrank(USDC_WHALE);
        uint256 amountToFund = 1000 * 10 ** 6; // 1000 USDC
        IERC20(USDC_ADDR).transfer(address(inputAccount), amountToFund);
        vm.stopPrank();

        vm.startPrank(owner);

        // Deploy a new CCTPTransfer contract
        CCTPTransfer.CCTPTransferConfig memory validConfig = CCTPTransfer.CCTPTransferConfig({
            amount: amount,
            mintRecipient: bytes32(uint256(0x3)),
            inputAccount: inputAccount,
            destinationDomain: 5,
            cctpTokenMessenger: ITokenMessenger(CCTP_TOKEN_MESSENGER_ADDR),
            transferToken: USDC_ADDR
        });
        bytes memory configBytes = abi.encode(validConfig);
        cctpTransfer = new CCTPTransfer(owner, processor, configBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(cctpTransfer));

        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 balanceBefore = IERC20(USDC_ADDR).balanceOf(address(inputAccount));

        // Finally let's call the transfer function from the processor
        vm.prank(processor);
        cctpTransfer.transfer();

        // Get the balance after the transfer of the inputAccount
        uint256 balanceAfter = IERC20(USDC_ADDR).balanceOf(address(inputAccount));

        // Assert that the balance of the inputAccount has decreased by the amount
        assert(balanceBefore - balanceAfter == amount);
    }
}
