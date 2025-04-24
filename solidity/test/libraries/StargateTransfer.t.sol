// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {StargateTransfer} from "../../src/libraries/StargateTransfer.sol";
import {IStargate} from "@stargatefinance/stg-evm-v2/src/interfaces/IStargate.sol";
import {MockStargate} from "../mocks/MockStargate.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract StargateTransferTest is Test {
    // Contract under test
    StargateTransfer public stargateTransfer;

    // Mock contracts
    MockStargate public mockStargateNative;
    MockStargate public mockStargateERC20;
    BaseAccount public inputAccount;

    // Test addresses
    address public owner;
    address public processor;
    address public recipient;
    MockERC20 public testToken;

    // Setup function to initialize test environment
    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");
        recipient = makeAddr("recipient");
        testToken = new MockERC20("USDC", "USDC", 18);

        // Create mock account
        vm.prank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));

        // Create mock Stargate contracts
        mockStargateNative = new MockStargate(address(0));
        mockStargateERC20 = new MockStargate(address(testToken));

        // Deploy StargateTransfer contract
        vm.startPrank(owner);
        stargateTransfer = new StargateTransfer(owner, processor, encodeNativeConfig());
        inputAccount.approveLibrary(address(stargateTransfer));
        vm.stopPrank();
    }

    // Helper function to encode native token transfer configuration
    function encodeNativeConfig() internal view returns (bytes memory) {
        StargateTransfer.StargateTransferConfig memory config = StargateTransfer.StargateTransferConfig({
            recipient: bytes32(uint256(uint160(recipient))),
            inputAccount: inputAccount,
            destinationDomain: 1,
            stargateAddress: mockStargateNative,
            transferToken: address(0),
            amount: 1 ether,
            minAmountToReceive: 0,
            refundAddress: address(0),
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });
        return abi.encode(config);
    }

    // Helper function to encode ERC20 token transfer configuration
    function encodeERC20Config() internal view returns (bytes memory) {
        StargateTransfer.StargateTransferConfig memory config = StargateTransfer.StargateTransferConfig({
            recipient: bytes32(uint256(uint160(recipient))),
            inputAccount: inputAccount,
            destinationDomain: 1,
            stargateAddress: mockStargateERC20,
            transferToken: address(testToken),
            amount: 100 * 10 ** 18,
            minAmountToReceive: 0,
            refundAddress: address(0),
            extraOptions: "",
            composeMsg: "",
            oftCmd: ""
        });
        return abi.encode(config);
    }

    // Test configuration validation
    function testConfigValidation() public {
        // Test invalid input account
        bytes memory invalidConfig = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: BaseAccount(payable(address(0))),
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 0,
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        stargateTransfer.updateConfig(invalidConfig);

        // Test invalid stargate address
        invalidConfig = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: IStargate(address(0)),
                transferToken: address(0),
                amount: 0,
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        vm.expectRevert("Stargate address can't be zero address");
        stargateTransfer.updateConfig(invalidConfig);
    }

    function testUpdateConfigWithInvalidToken() public {
        MockERC20 differentToken = new MockERC20("Different", "DIFF", 18);

        bytes memory invalidConfig = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateERC20, // Uses testToken
                transferToken: address(differentToken), // Different token
                amount: 100 * 10 ** 18,
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        vm.expectRevert("Token address does not match the stargate token address");
        stargateTransfer.updateConfig(invalidConfig);
    }

    function testTransferWithDustAmount() public {
        // Prepare the account with a tiny amount of native tokens
        vm.deal(address(inputAccount), 0.0001 ether);

        // Update config with a very small amount
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 0.00001 ether, // Tiny amount
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        vm.expectRevert("Insufficient balance for transfer and fees");
        stargateTransfer.transfer();
    }

    function testTransferExactNativeBalance() public {
        // Scenario: Transfer the entire native balance, which requires precise fee calculation

        // Calculate a balance that should just cover the transfer and fees
        uint256 transferAmount = 1 ether;

        // Deal the exact balance needed for transfer
        vm.deal(address(inputAccount), transferAmount);

        // Create config to transfer full balance
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 0, // Transfer full balance
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();

        // Verify that the input account's balance is now zero
        assertEq(address(inputAccount).balance, 0, "Account balance should be zero after full transfer");
    }

    // Test native token transfer with full balance
    function testNativeTokenTransferFullBalance() public {
        // Prepare the account with native tokens
        vm.deal(address(inputAccount), 10 ether);

        // Update config for full balance transfer
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 0, // Transfer full balance
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test native token transfer with specific amount
    function testNativeTokenTransferSpecificAmount() public {
        // Prepare the account with native tokens
        vm.deal(address(inputAccount), 10 ether);

        // Update config with specific amount
        bytes memory config = encodeNativeConfig();

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test ERC20 token transfer with full balance
    function testERC20TokenTransferFullBalance() public {
        // Mint tokens to the mock account
        uint256 fullBalance = 200 * 10 ** 18;
        vm.prank(owner);
        testToken.mint(address(inputAccount), fullBalance);

        // Prepare account with native tokens for fees
        vm.deal(address(inputAccount), 1 ether);

        // Update config for full balance transfer
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateERC20,
                transferToken: address(testToken),
                amount: 0, // Transfer full balance
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test ERC20 token transfer with specific amount
    function testERC20TokenTransferSpecificAmount() public {
        // Mint tokens to the mock account
        uint256 fullBalance = 200 * 10 ** 18;
        vm.prank(owner);
        testToken.mint(address(inputAccount), fullBalance);

        // Prepare account with native tokens for fees
        vm.deal(address(inputAccount), 1 ether);

        // Update config with specific amount
        bytes memory config = encodeERC20Config();

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test transfer with custom refund address
    function testTransferWithCustomRefundAddress() public {
        address customRefund = makeAddr("customRefund");

        // Prepare the account with native tokens
        vm.deal(address(inputAccount), 10 ether);

        // Update config with custom refund address
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 1 ether,
                minAmountToReceive: 0,
                refundAddress: customRefund,
                extraOptions: "",
                composeMsg: "",
                oftCmd: ""
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test transfer failure scenarios
    function testTransferFailures() public {
        // Native token transfer with insufficient balance
        vm.deal(address(inputAccount), 0.0005 ether); // Not enough for transfer + fees

        bytes memory config = encodeNativeConfig();
        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        vm.prank(processor);
        vm.expectRevert("Insufficient balance for transfer and fees");
        stargateTransfer.transfer();

        // ERC20 token transfer with insufficient native balance for fees
        vm.deal(address(inputAccount), 0); // No native balance for fees

        config = encodeERC20Config();
        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        vm.prank(processor);
        vm.expectRevert("Insufficient native balance for fees");
        stargateTransfer.transfer();
    }

    // Test transfer with custom options and compose message
    function testTransferWithCustomOptions() public {
        // Prepare the account with native tokens
        vm.deal(address(inputAccount), 10 ether);

        // Update config with custom options and compose message
        bytes memory config = abi.encode(
            StargateTransfer.StargateTransferConfig({
                recipient: bytes32(uint256(uint160(recipient))),
                inputAccount: inputAccount,
                destinationDomain: 1,
                stargateAddress: mockStargateNative,
                transferToken: address(0),
                amount: 1 ether,
                minAmountToReceive: 0,
                refundAddress: address(0),
                extraOptions: hex"01", // Example custom option
                composeMsg: hex"02", // Example compose message
                oftCmd: hex"03" // Example OFT command
            })
        );

        vm.prank(owner);
        stargateTransfer.updateConfig(config);

        // Execute transfer as processor
        vm.prank(processor);
        stargateTransfer.transfer();
    }

    // Test unauthorized access
    function testUnauthorizedAccess() public {
        address nonOwner = address(3);
        vm.startPrank(nonOwner);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, nonOwner));
        stargateTransfer.updateConfig(encodeNativeConfig());
    }
}
