// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {IBCEurekaTransfer} from "../../src/libraries/IBCEurekaTransfer.sol";
import {IEurekaHandler} from "../../src/libraries/interfaces/eureka/IEurekaHandler.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

/**
 * @title IBCEurekaTransfer Test
 * @dev Test suite for IBCEurekaTransfer contract functionality
 */
contract IBCEurekaTransferTest is Test {
    IBCEurekaTransfer public ibcEurekaTransfer;
    BaseAccount inputAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);
    address eurekaHandler = address(3);
    address feeRecipient = address(4);
    IEurekaHandler mockEurekaHandler;

    string recipient = "cosmos1xyz..."; // Example bech32 address
    string sourceClient = "cosmoshub-4";
    uint64 timeout = 600; // 10 minutes

    /**
     * @dev Setup test environment
     * Deploys token, input account, a mock Eureka handler and an IBCEurekaTransfer contract with initial config
     */
    function setUp() public {
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        token = new MockERC20("TEST", "TEST", 18);
        mockEurekaHandler = IEurekaHandler(eurekaHandler);

        // Create a valid configuration for token transfer
        IBCEurekaTransfer.IBCEurekaTransferConfig memory validConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 1000,
            transferToken: address(token),
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeout,
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(validConfig);
        ibcEurekaTransfer = new IBCEurekaTransfer(owner, processor, configBytes);
        inputAccount.approveLibrary(address(ibcEurekaTransfer));

        vm.stopPrank();
    }

    function testUpdateConfigFailsZeroEurekaHandler() public {
        IBCEurekaTransfer.IBCEurekaTransferConfig memory invalidConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 1000,
            transferToken: address(token),
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeout,
            eurekaHandler: IEurekaHandler(address(0)) // Zero address (invalid)
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Eureka Handler can't be zero address");
        ibcEurekaTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroTransferToken() public {
        IBCEurekaTransfer.IBCEurekaTransferConfig memory invalidConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 1000,
            transferToken: address(0), // Zero address (invalid)
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeout,
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Transfer token can't be zero address");
        ibcEurekaTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroInputAccount() public {
        IBCEurekaTransfer.IBCEurekaTransferConfig memory invalidConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 1000,
            transferToken: address(token),
            inputAccount: BaseAccount(payable(address(0))), // Zero address (invalid)
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeout,
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        ibcEurekaTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroTimeout() public {
        IBCEurekaTransfer.IBCEurekaTransferConfig memory invalidConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 1000,
            transferToken: address(token),
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: 0, // Zero timeout (invalid)
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Timeout can't be zero");
        ibcEurekaTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigSucceeds() public {
        IBCEurekaTransfer.IBCEurekaTransferConfig memory validConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 2000, // Different amount
            transferToken: address(token),
            inputAccount: inputAccount,
            recipient: "cosmos1abc...", // Different recipient
            sourceClient: "osmosis-1", // Different source client
            timeout: 1200, // Different timeout
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(validConfig);
        vm.prank(owner);
        ibcEurekaTransfer.updateConfig(configBytes);

        // Verify config was updated successfully
        (
            uint256 newAmount,
            address newToken,
            BaseAccount newAccount,
            string memory newRecipient,
            string memory newSourceClient,
            uint64 newTimeout,
        ) = ibcEurekaTransfer.config();

        assertEq(newAmount, 2000, "Amount should be updated");
        assertEq(newToken, address(token), "Token should be unchanged");
        assertEq(address(newAccount), address(inputAccount), "Account should be unchanged");
        assertEq(keccak256(bytes(newRecipient)), keccak256(bytes("cosmos1abc...")), "Recipient should be updated");
        assertEq(keccak256(bytes(newSourceClient)), keccak256(bytes("osmosis-1")), "Source client should be updated");
        assertEq(newTimeout, 1200, "Timeout should be updated");
    }

    function testTransferFailsNoTokenBalance() public {
        // No tokens provided to the input account

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 100,
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        vm.expectRevert("Nothing to transfer");
        ibcEurekaTransfer.transfer(fees, "");
    }

    function testTransferFailsInsufficientTokenBalance() public {
        // Mint less than the required amount of tokens
        token.mint(address(inputAccount), 500);

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 100,
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        ibcEurekaTransfer.transfer(fees, "");
    }

    function testTransferFailsNotEnoughToPayFees() public {
        // Mint enough tokens but set high fees
        token.mint(address(inputAccount), 1500);

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 1600, // More than available balance
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        vm.expectRevert("Not enough to pay fees and make a transfer");
        ibcEurekaTransfer.transfer(fees, "");
    }

    function testTransferFailsWithEqualFeesAndAmount() public {
        // Mint exactly the amount of tokens, but fees consume it all
        token.mint(address(inputAccount), 1000);

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 1000, // Equal to the amount to transfer
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        vm.expectRevert("Not enough to pay fees and make a transfer");
        ibcEurekaTransfer.transfer(fees, "");
    }

    function testTransferSucceedsWithSufficientBalance() public {
        // Mint more than the required amount of tokens
        token.mint(address(inputAccount), 1500);

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 200,
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        // This call should succeed because the balance is sufficient
        ibcEurekaTransfer.transfer(fees, "test memo");
    }

    function testTransferSucceedsWithFullAmount() public {
        // Update config to transfer full token balance
        IBCEurekaTransfer.IBCEurekaTransferConfig memory fullConfig = IBCEurekaTransfer.IBCEurekaTransferConfig({
            amount: 0, // Transfer full balance
            transferToken: address(token),
            inputAccount: inputAccount,
            recipient: recipient,
            sourceClient: sourceClient,
            timeout: timeout,
            eurekaHandler: mockEurekaHandler
        });

        bytes memory configBytes = abi.encode(fullConfig);
        vm.prank(owner);
        ibcEurekaTransfer.updateConfig(configBytes);

        // Mint some tokens to the input account
        token.mint(address(inputAccount), 500);

        IEurekaHandler.Fees memory fees = IEurekaHandler.Fees({
            relayFee: 100,
            relayFeeRecipient: feeRecipient,
            quoteExpiry: uint64(block.timestamp + 3600)
        });

        vm.prank(processor);
        // This call should succeed and transfer the full balance minus fees
        ibcEurekaTransfer.transfer(fees, "");
    }
}
