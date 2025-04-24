// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {StandardBridgeTransfer} from "../../src/libraries/StandardBridgeTransfer.sol";
import {IStandardBridge} from "../../src/libraries/interfaces/standard-bridge/IStandardBridge.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

/**
 * @title StandardBridgeTransfer Test
 * @dev Test suite for StandardBridgeTransfer contract functionality
 */
contract StandardBridgeTransferTest is Test {
    StandardBridgeTransfer public standardBridgeTransfer;
    BaseAccount inputAccount;
    MockERC20 token;
    MockERC20 remoteToken;

    address owner = address(1);
    address processor = address(2);
    address recipient = address(3);
    address bridge = address(4);
    IStandardBridge mockStandardBridge;
    uint32 minGasLimit = 200000;

    /**
     * @dev Setup test environment
     * Deploys token, input account, a mock standard bridge and a StandardBridgeTransfer contract with initial config
     */
    function setUp() public {
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        token = new MockERC20("TEST", "TEST", 18);
        remoteToken = new MockERC20("REMOTE", "REMOTE", 18);
        mockStandardBridge = IStandardBridge(address(bridge));

        // Create a valid configuration for ERC20 transfer
        StandardBridgeTransfer.StandardBridgeTransferConfig memory validConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(token),
            remoteToken: address(remoteToken),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(validConfig);
        standardBridgeTransfer = new StandardBridgeTransfer(owner, processor, configBytes);
        inputAccount.approveLibrary(address(standardBridgeTransfer));

        vm.stopPrank();
    }

    function testUpdateConfigFailsZeroStandardBridge() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory invalidConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: IStandardBridge(payable(address(0))), // Zero address (invalid)
            token: address(token),
            remoteToken: address(remoteToken),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("StandardBridge can't be zero address");
        standardBridgeTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroRecipient() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory invalidConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: address(0), // Zero address (invalid)
            standardBridge: mockStandardBridge,
            token: address(token),
            remoteToken: address(remoteToken),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Recipient can't be zero address");
        standardBridgeTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroInputAccount() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory invalidConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: BaseAccount(payable(address(0))), // Zero address (invalid)
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(token),
            remoteToken: address(remoteToken),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        standardBridgeTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroRemoteTokenForERC20() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory invalidConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(token), // Non-zero token
            remoteToken: address(0), // Zero remote token (invalid for ERC20)
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Remote token must be specified for ERC20 transfers");
        standardBridgeTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsRemoteTokenForETH() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory invalidConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(remoteToken), // Non-zero remote token (invalid for ETH)
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Remote token must not be specified for ETH transfers");
        standardBridgeTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigSucceedsWithZeroRemoteTokenForETH() public {
        StandardBridgeTransfer.StandardBridgeTransferConfig memory validConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(0), // Zero remote token (valid for ETH)
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(validConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // Verify config was updated successfully
        (,,,, address newToken, address newRemoteToken,,) = standardBridgeTransfer.config();
        assertEq(newToken, address(0), "Token should be ETH");
        assertEq(newRemoteToken, address(0), "Remote token should be ETH");
    }

    function testTransferFailsNoETHBalance() public {
        // Update config to use ETH instead of ERC20
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(0),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(ethConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // No ETH balance provided
        vm.prank(processor);
        vm.expectRevert("No balance to transfer");
        standardBridgeTransfer.transfer();
    }

    function testTransferFailsInsufficientETHBalance() public {
        // Update config to use ETH instead of ERC20
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(0),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(ethConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // Fund the account with less than the required amount
        vm.deal(address(inputAccount), 500);

        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        standardBridgeTransfer.transfer();
    }

    function testTransferFailsNoERC20Balance() public {
        // No tokens provided
        vm.prank(processor);
        vm.expectRevert("No balance to transfer");
        standardBridgeTransfer.transfer();
    }

    function testTransferFailsInsufficientERC20Balance() public {
        // Mint less than the required amount of tokens
        token.mint(address(inputAccount), 500);
        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        standardBridgeTransfer.transfer();
    }

    function testTransferSucceedsWithSufficientETHBalance() public {
        // Update config to use ETH instead of ERC20
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 1000,
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(0),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(ethConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // Fund the account with sufficient ETH
        vm.deal(address(inputAccount), 1500);

        vm.prank(processor);
        // This call should succeed because the balance is sufficient
        standardBridgeTransfer.transfer();
    }

    function testTransferSucceedsWithSufficientERC20Balance() public {
        // Mint more than the required amount of tokens
        token.mint(address(inputAccount), 1500);
        vm.prank(processor);
        // This call should succeed because the balance is sufficient
        standardBridgeTransfer.transfer();
    }

    function testTransferSucceedsWithFullETHAmount() public {
        // Update config to use ETH with amount=0 (transfer full balance)
        StandardBridgeTransfer.StandardBridgeTransferConfig memory ethConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 0, // Transfer full balance
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(0), // ETH transfer
            remoteToken: address(0),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(ethConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // Fund the account with some ETH
        vm.deal(address(inputAccount), 500);

        vm.prank(processor);
        // This call should succeed and transfer the full balance
        standardBridgeTransfer.transfer();
    }

    function testTransferSucceedsWithFullERC20Amount() public {
        // Update config to transfer full token balance
        StandardBridgeTransfer.StandardBridgeTransferConfig memory fullConfig = StandardBridgeTransfer
            .StandardBridgeTransferConfig({
            amount: 0, // Transfer full balance
            inputAccount: inputAccount,
            recipient: recipient,
            standardBridge: mockStandardBridge,
            token: address(token),
            remoteToken: address(remoteToken),
            minGasLimit: minGasLimit,
            extraData: bytes("")
        });
        bytes memory configBytes = abi.encode(fullConfig);
        vm.prank(owner);
        standardBridgeTransfer.updateConfig(configBytes);

        // Mint some tokens to the input account
        token.mint(address(inputAccount), 500);

        vm.prank(processor);
        // This call should succeed and transfer the full balance
        standardBridgeTransfer.transfer();
    }
}
