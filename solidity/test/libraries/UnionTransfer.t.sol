// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {UnionTransfer} from "../../src/libraries/UnionTransfer.sol";
import {IUnion} from "../../src/libraries/interfaces/union/IUnion.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {console} from "forge-std/src/console.sol";

/**
 * @title UnionTransfer Test
 * @dev Test suite for UnionTransfer contract functionality
 */
contract UnionTransferTest is Test {
    UnionTransfer public unionTransfer;
    BaseAccount inputAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);
    address zkGMAddress = address(3);
    IUnion mockZkGM;

    // Example values for testing
    bytes recipient = hex"62626e31346d6c706434386b35766b657365743478376637386d797a336d34376a6361787a3967706e6c"; // Example bech32 address in bytes
    bytes transferToken;
    bytes quoteToken =
        hex"62626e31333030736530767775653737686e36733877706836346579366435357a616634386a72766567397761667371756e636e33653473637373677664"; // Example quote token
    uint32 channelId = 123;
    uint64 timeout = 259200; // 3 days

    /**
     * @dev Setup test environment
     * Deploys token, input account, a mock zkGM and a UnionTransfer contract with initial config
     */
    function setUp() public {
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        token = new MockERC20("TEST", "TEST");
        mockZkGM = IUnion(zkGMAddress);

        // Convert token address to bytes for the config
        transferToken = abi.encodePacked(address(token));

        // Create a valid configuration for token transfer
        UnionTransfer.UnionTransferConfig memory validConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(validConfig);
        unionTransfer = new UnionTransfer(owner, processor, configBytes);
        inputAccount.approveLibrary(address(unionTransfer));

        vm.stopPrank();
    }

    function testUpdateConfigFailsZeroZkGM() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: IUnion(address(0)), // Zero address (invalid)
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("zkGM can't be zero address");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsEmptyTransferToken() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: new bytes(0), // Empty bytes (invalid)
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Transfer token can't be empty bytes");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroInputAccount() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: BaseAccount(payable(address(0))), // Zero address (invalid)
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsEmptyRecipient() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: new bytes(0), // Empty bytes (invalid)
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Recipient can't be empty bytes");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsEmptyTransferTokenName() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "", // Empty string (invalid)
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Transfer token name can't be empty");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsEmptyTransferTokenSymbol() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "", // Empty string (invalid)
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Transfer token symbol can't be empty");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsEmptyQuoteToken() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: new bytes(0), // Empty bytes (invalid)
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Quote token can't be empty bytes");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroTimeout() public {
        UnionTransfer.UnionTransferConfig memory invalidConfig = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 980,
            channelId: channelId,
            timeout: 0 // Zero timeout (invalid)
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Timeout can't be zero");
        unionTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigSucceeds() public {
        bytes memory newRecipient = hex"62626e31616263646566"; // Different recipient
        bytes memory newQuoteToken = hex"7562626e"; // Different quote token

        UnionTransfer.UnionTransferConfig memory validConfig = UnionTransfer.UnionTransferConfig({
            amount: 2000, // Different amount
            inputAccount: inputAccount,
            recipient: newRecipient,
            protocolVersion: 2, // Different version
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "NEW_TEST", // Different name
            transferTokenSymbol: "NTEST", // Different symbol
            transferTokenDecimals: 8, // Different decimals
            transferTokenUnwrappingPath: 2, // Different path
            quoteToken: newQuoteToken,
            quoteTokenAmount: 1950, // Different quote amount
            channelId: 456, // Different channel
            timeout: 86400 // Different timeout
        });

        bytes memory configBytes = abi.encode(validConfig);
        vm.prank(owner);
        unionTransfer.updateConfig(configBytes);

        // Verify config was updated successfully
        (
            uint256 newAmount,
            BaseAccount newInputAccount,
            bytes memory newRecipientBytes,
            uint8 newProtocolVersion,
            IUnion newZkGM,
            ,
            string memory newTransferTokenName,
            string memory newTransferTokenSymbol,
            uint8 newTransferTokenDecimals,
            uint256 newTransferTokenUnwrappingPath,
            bytes memory newQuoteTokenBytes,
            uint256 newQuoteTokenAmount,
            uint32 newChannelId,
            uint64 newTimeout
        ) = unionTransfer.config();

        assertEq(newAmount, 2000, "Amount should be updated");
        assertEq(address(newInputAccount), address(inputAccount), "Account should be unchanged");
        assertEq(keccak256(newRecipientBytes), keccak256(newRecipient), "Recipient should be updated");
        assertEq(newProtocolVersion, 2, "Protocol version should be updated");
        assertEq(address(newZkGM), zkGMAddress, "zkGM should be unchanged");
        assertEq(
            keccak256(abi.encodePacked(newTransferTokenName)),
            keccak256(abi.encodePacked("NEW_TEST")),
            "Transfer token name should be updated"
        );
        assertEq(
            keccak256(abi.encodePacked(newTransferTokenSymbol)),
            keccak256(abi.encodePacked("NTEST")),
            "Transfer token symbol should be updated"
        );
        assertEq(newTransferTokenDecimals, 8, "Transfer token decimals should be updated");
        assertEq(newTransferTokenUnwrappingPath, 2, "Transfer token unwrapping path should be updated");
        assertEq(keccak256(newQuoteTokenBytes), keccak256(newQuoteToken), "Quote token should be updated");
        assertEq(newQuoteTokenAmount, 1950, "Quote token amount should be updated");
        assertEq(newChannelId, 456, "Channel ID should be updated");
        assertEq(newTimeout, 86400, "Timeout should be updated");
    }

    function testTransferFailsNoTokenBalance() public {
        // No tokens provided to the input account
        vm.prank(processor);
        vm.expectRevert("Nothing to transfer");
        unionTransfer.transfer(0);
    }

    function testTransferFailsInsufficientTokenBalance() public {
        // Mint less than the required amount of tokens
        token.mint(address(inputAccount), 500);

        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        unionTransfer.transfer(0);
    }

    function testTransferSucceedsWithSufficientBalance() public {
        // Mock the token for proper ERC20 behavior
        token.mint(address(inputAccount), 1500);

        vm.prank(processor);
        // This call should succeed because the balance is sufficient
        unionTransfer.transfer(0);
    }

    function testTransferWithCustomQuoteAmount() public {
        // Mint tokens
        token.mint(address(inputAccount), 1500);

        vm.prank(processor);
        // Transfer with custom quote amount
        unionTransfer.transfer(900);
    }

    function testTransferSucceedsWithFullAmount() public {
        // Update config to transfer full token balance
        UnionTransfer.UnionTransferConfig memory fullConfig = UnionTransfer.UnionTransferConfig({
            amount: 0, // Transfer full balance
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 0, // Will match the full amount
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(fullConfig);
        vm.prank(owner);
        unionTransfer.updateConfig(configBytes);

        // Mint some tokens to the input account
        token.mint(address(inputAccount), 500);

        vm.prank(processor);
        // This call should succeed and transfer the full balance
        unionTransfer.transfer(0);
    }

    function testTransferWithNonZeroQuoteAmountButZeroConfigQuoteAmount() public {
        // Update config with zero quoteTokenAmount
        UnionTransfer.UnionTransferConfig memory configWithZeroQuote = UnionTransfer.UnionTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            protocolVersion: 1,
            zkGM: mockZkGM,
            transferToken: transferToken,
            transferTokenName: "TEST",
            transferTokenSymbol: "TEST",
            transferTokenDecimals: 18,
            transferTokenUnwrappingPath: 1,
            quoteToken: quoteToken,
            quoteTokenAmount: 0, // Zero in config
            channelId: channelId,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(configWithZeroQuote);
        vm.prank(owner);
        unionTransfer.updateConfig(configBytes);

        // Mint tokens
        token.mint(address(inputAccount), 1500);

        vm.prank(processor);
        // Pass a non-zero quote amount during transfer call
        unionTransfer.transfer(950);
    }

    function testTransferCounterIncrement() public {
        // This test verifies that the counter increases with each transfer
        // Mint tokens
        token.mint(address(inputAccount), 2000);

        // Call transfer multiple times
        vm.startPrank(processor);
        unionTransfer.transfer(0);
        // Check the counter value
        uint256 counterAfterFirstTransfer = unionTransfer.counter();
        assertEq(counterAfterFirstTransfer, 1, "Counter should be 1 after first transfer");
        unionTransfer.transfer(0);
        // Check the counter value again
        uint256 counterAfterSecondTransfer = unionTransfer.counter();
        assertEq(counterAfterSecondTransfer, 2, "Counter should be 2 after second transfer");
        vm.stopPrank();
    }

    function testDecodeFungibleAssetOrderTest() public pure {
        // Bytes taken from a real transaction on Etherscan for the operand field
        bytes memory fungibleAssetOrder =
            hex"0000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000001e00000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000000000022000000000000000000000000000000000000000000000000000000000000002600000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000000000001441568848e805c9fed20494e35669f8b0110db7a9000000000000000000000000000000000000000000000000000000000000000000000000000000000000002a62626e3133377577716d733375616a727a726a646778706a357368746175617a786d3277786434706676000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014e53dcec07d16d88e386ae0710e86d9a400f83c31000000000000000000000000000000000000000000000000000000000000000000000000000000000000000442414259000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007426162796c6f6e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000047562626e00000000000000000000000000000000000000000000000000000000";

        // Decode field by field
        (
            bytes memory sender,
            bytes memory receiver,
            bytes memory baseToken,
            uint256 baseAmount,
            string memory baseTokenSymbol,
            string memory baseTokenName,
            uint8 baseTokenDecimals,
            uint256 baseTokenPath,
            bytes memory quoteTokenReturned,
            uint256 quoteAmount
        ) = abi.decode(
            fungibleAssetOrder, (bytes, bytes, bytes, uint256, string, string, uint8, uint256, bytes, uint256)
        );

        // Construct the struct from the decoded fields
        IUnion.FungibleAssetOrder memory decodedOrder = IUnion.FungibleAssetOrder({
            sender: sender,
            receiver: receiver,
            baseToken: baseToken,
            baseAmount: baseAmount,
            baseTokenSymbol: baseTokenSymbol,
            baseTokenName: baseTokenName,
            baseTokenDecimals: baseTokenDecimals,
            baseTokenPath: baseTokenPath,
            quoteToken: quoteTokenReturned,
            quoteAmount: quoteAmount
        });

        // Print the result
        printOrderDetails(decodedOrder);
    }

    // Helper function to print order details
    function printOrderDetails(IUnion.FungibleAssetOrder memory order) internal pure {
        console.log("Sender:");
        console.logBytes(order.sender);
        console.log("Receiver:");
        console.logBytes(order.receiver);
        console.log("Base Token:");
        console.logBytes(order.baseToken);
        console.log("Base Amount:");
        console.log(order.baseAmount);
        console.log("Base Token Symbol:");
        console.log(order.baseTokenSymbol);
        console.log("Base Token Name:");
        console.log(order.baseTokenName);
        console.log("Base Token Decimals:");
        console.log(uint256(order.baseTokenDecimals));
        console.log("Base Token Path:");
        console.log(order.baseTokenPath);
        console.log("Quote Token:");
        console.logBytes(order.quoteToken);
        console.log("Quote Amount:");
        console.log(order.quoteAmount);
    }
}
