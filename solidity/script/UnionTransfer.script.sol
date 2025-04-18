// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script} from "forge-std/src/Script.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {UnionTransfer} from "../src/libraries/UnionTransfer.sol";
import {IUnion} from "../src/libraries/interfaces/union/IUnion.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {console} from "forge-std/src/console.sol";

contract UnionTransferScript is Script {
    // Address of the UBBN ERC-20 token on Ethereum
    address constant UBBN_ADDR = 0xe53dCec07d16D88e386AE0710E86d9a400f83c31;

    // Address of the zkGM (Union UCS03-ZKGM protocol) on Ethereum
    address constant ZKGM_ADDR = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;

    // Address of UBBN WHALE
    address constant UBBN_WHALE = 0x24C31E2d6c07b1290933924abF5c97A4234600ff;

    // Example addresses for owner and processor
    address owner = address(1);
    address processor = address(2);

    // Contracts
    UnionTransfer public unionTransfer;
    UnionTransfer public unionTransferFull;
    BaseAccount inputAccount;

    // Amount to transfer
    uint256 tokenAmount = 100000000; // For UBBN
    uint64 timeoutTimestamp = 259200; // 3 days timeout

    // Babylon recipient address in bytes format
    bytes recipient = hex"62626e31346d6c706434386b35766b657365743478376637386d797a336d34376a6361787a3967706e6c"; // Example bech32 address in bytes

    function run() external {
        // Create a fork of Ethereum mainnet and switch to it
        uint256 forkId = vm.createFork("https://eth-mainnet.public.blastapi.io");
        vm.selectFork(forkId);

        // Start broadcasting transactions
        vm.startPrank(owner);

        // Deploy a new BaseAccount contract
        inputAccount = new BaseAccount(owner, new address[](0));

        // Convert UBBN_ADDR to bytes for configuration
        bytes memory transferToken = abi.encodePacked(UBBN_ADDR);

        // Quote token for example (Babylon native token in bytes)
        bytes memory quoteToken = hex"7562626e"; // "ubbn" in bytes

        // Deploy a new UnionTransfer contract with fixed amount
        UnionTransfer.UnionTransferConfig memory ubbnConfig = UnionTransfer.UnionTransferConfig({
            amount: tokenAmount,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: IUnion(ZKGM_ADDR),
            transferToken: transferToken,
            transferTokenName: "Babylon",
            transferTokenSymbol: "BABY",
            transferTokenDecimals: 6,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: tokenAmount,
            channelId: 1,
            timeout: timeoutTimestamp
        });

        bytes memory ubbnConfigBytes = abi.encode(ubbnConfig);
        unionTransfer = new UnionTransfer(owner, processor, ubbnConfigBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(unionTransfer));

        vm.stopPrank();

        // Fund the input account with UBBN tokens
        vm.startPrank(UBBN_WHALE);
        IERC20 ubbn = IERC20(UBBN_ADDR);
        ubbn.transfer(address(inputAccount), tokenAmount * 2);
        vm.stopPrank();

        // Get the balance before the transfer of the inputAccount
        uint256 ubbnBalanceBefore = ubbn.balanceOf(address(inputAccount));
        console.log("UBBN balance before transfer: ", ubbnBalanceBefore);

        // Execute the UBBN transfer with default quote amount (0 means use config value)
        vm.prank(processor);
        unionTransfer.transfer(0);

        // Get the balance after the transfer
        uint256 ubbnBalanceAfter = ubbn.balanceOf(address(inputAccount));
        console.log("UBBN balance after transfer: ", ubbnBalanceAfter);

        // Check balance changes
        console.log("UBBN sent: ", ubbnBalanceBefore - ubbnBalanceAfter);

        // Verify that the balances have been correctly deducted (tokenAmount)
        assert(ubbnBalanceBefore - ubbnBalanceAfter == tokenAmount);

        // Deploy a new UnionTransfer contract for full balance transfer
        vm.startPrank(owner);
        UnionTransfer.UnionTransferConfig memory fullBalanceConfig = UnionTransfer.UnionTransferConfig({
            amount: 0, // 0 means transfer full balance
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: IUnion(ZKGM_ADDR),
            transferToken: transferToken,
            transferTokenName: "Babylon",
            transferTokenSymbol: "BABY",
            transferTokenDecimals: 6,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 0, // 0 means same as transfer amount
            channelId: 1,
            timeout: timeoutTimestamp
        });

        bytes memory fullBalanceConfigBytes = abi.encode(fullBalanceConfig);
        unionTransferFull = new UnionTransfer(owner, processor, fullBalanceConfigBytes);

        // Approve the library from the input account
        inputAccount.approveLibrary(address(unionTransferFull));
        vm.stopPrank();

        uint256 ubbnBalanceBeforeFullTransfer = ubbn.balanceOf(address(inputAccount));
        console.log("UBBN balance before full transfer: ", ubbnBalanceBeforeFullTransfer);

        // Execute the UBBN transfer for full amount with custom quote amount
        vm.prank(processor);
        unionTransferFull.transfer(ubbnBalanceBeforeFullTransfer * 2); // Request 2x tokens on the other side

        uint256 ubbnBalanceAfterFullTransfer = ubbn.balanceOf(address(inputAccount));
        console.log("UBBN balance after full transfer: ", ubbnBalanceAfterFullTransfer);

        // Check that all UBBN has been transferred
        assert(ubbnBalanceAfterFullTransfer == 0);
    }
}
