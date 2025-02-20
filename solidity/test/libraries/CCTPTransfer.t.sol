// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {CCTPTransfer} from "../../src/libraries/CCTPTransfer.sol";
import {ITokenMessenger} from "../../src/libraries/interfaces/cctp/ITokenMessenger.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {Account as ValenceAccount} from "../../src/accounts/Account.sol";
import {MockTokenMessenger} from "../mocks/MockTokenMessenger.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

/**
 * @title CCTPTransfer Test
 * @dev Test suite for CCTPTransfer contract functionality
 */
contract CCTPTransferTest is Test {
    CCTPTransfer public cctpTransfer;
    BaseAccount inputAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);
    bytes32 public mintRecipient = bytes32(uint256(0x3));
    uint32 public destinationDomain = 100;
    ITokenMessenger stubTokenMessenger = new MockTokenMessenger();

    /**
     * @dev Setup test environment
     * Deploys token, input account, a stub token messenger without the functionality and a CCTP transfer contract with initial config
     */
    function setUp() public {
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        token = new MockERC20("USDC", "USDC");

        // Create a valid configuration.
        CCTPTransfer.CCTPTransferConfig memory validConfig = CCTPTransfer.CCTPTransferConfig({
            amountToTransfer: 1000,
            mintRecipient: mintRecipient,
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            cctpTokenMessenger: ITokenMessenger(address(stubTokenMessenger)),
            transferToken: address(token)
        });
        bytes memory configBytes = abi.encode(validConfig);
        cctpTransfer = new CCTPTransfer(owner, processor, configBytes);
        inputAccount.approveLibrary(address(cctpTransfer));

        vm.stopPrank();
    }

    function testUpdateConfigFailsZeroTokenMessenger() public {
        CCTPTransfer.CCTPTransferConfig memory invalidConfig = CCTPTransfer.CCTPTransferConfig({
            amountToTransfer: 100,
            mintRecipient: mintRecipient,
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            cctpTokenMessenger: ITokenMessenger(address(0)), // Zero address (invalid)
            transferToken: address(token)
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("CCTP Token Messenger can't be zero address");
        cctpTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroTransferToken() public {
        CCTPTransfer.CCTPTransferConfig memory invalidConfig = CCTPTransfer.CCTPTransferConfig({
            amountToTransfer: 100,
            mintRecipient: mintRecipient,
            inputAccount: inputAccount,
            destinationDomain: destinationDomain,
            cctpTokenMessenger: ITokenMessenger(address(stubTokenMessenger)),
            transferToken: address(0) // Zero address (invalid)
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Transfer token can't be zero address");
        cctpTransfer.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroInputAccount() public {
        CCTPTransfer.CCTPTransferConfig memory invalidConfig = CCTPTransfer.CCTPTransferConfig({
            amountToTransfer: 100,
            mintRecipient: mintRecipient,
            inputAccount: ValenceAccount(payable(address(0))), // Zero address (invalid)
            destinationDomain: destinationDomain,
            cctpTokenMessenger: ITokenMessenger(address(stubTokenMessenger)),
            transferToken: address(token)
        });
        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        cctpTransfer.updateConfig(configBytes);
    }

    function testTransferFailsInsufficientBalance() public {
        // Mint less than the required amount of tokens
        token.mint(address(inputAccount), 50);
        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        cctpTransfer.transfer();
    }

    function testTransferFailsNoBalance() public {
        vm.prank(processor);
        vm.expectRevert("Insufficient balance");
        cctpTransfer.transfer();
    }

    function testTransferSucceedsWithSufficientBalance() public {
        // Set the dummy token balance for the dummy account higher than the required amount.
        token.mint(address(inputAccount), 1500);
        vm.prank(processor);
        // This call should succeed because the balance is sufficient.
        cctpTransfer.transfer();
    }
}
