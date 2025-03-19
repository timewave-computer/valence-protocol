// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {Forwarder} from "../../src/libraries/Forwarder.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title Forwarder Test
 * @dev Test suite for Forwarder contract functionality
 */
contract ForwarderTest is Test {
    // Test contracts and addresses
    Forwarder forwarder;
    BaseAccount inputAccount;
    BaseAccount outputAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);

    /**
     * @dev Setup test environment
     * Deploys token, accounts and forwarder with initial config
     */
    function setUp() public {
        // Set initial block timestamp and height
        vm.warp(5000);
        vm.roll(100);

        vm.startPrank(owner);
        // Deploy mock contracts
        token = new MockERC20("Test Token", "TEST");

        // Create forwarder first
        Forwarder.ForwardingConfig[] memory fConfigs = new Forwarder.ForwardingConfig[](2);
        fConfigs[0] = Forwarder.ForwardingConfig(address(0), 1 ether);
        fConfigs[1] = Forwarder.ForwardingConfig(address(token), 100);

        // Create accounts after forwarder
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));

        Forwarder.ForwarderConfig memory config =
            Forwarder.ForwarderConfig(inputAccount, outputAccount, fConfigs, Forwarder.IntervalType.BLOCKS, 1);

        forwarder = new Forwarder(owner, processor, abi.encode(config));
        inputAccount.approveLibrary(address(forwarder));
        vm.stopPrank();
    }

    function testUpdateConfig() public {
        vm.startPrank(owner);

        Forwarder.ForwardingConfig[] memory newConfigs = new Forwarder.ForwardingConfig[](1);
        newConfigs[0] = Forwarder.ForwardingConfig(address(token), 200);

        Forwarder.ForwarderConfig memory newConfig =
            Forwarder.ForwarderConfig(inputAccount, outputAccount, newConfigs, Forwarder.IntervalType.TIME, 100);

        forwarder.updateConfig(abi.encode(newConfig));

        vm.stopPrank();
    }

    function testCannotUpdateConfigNonOwner() public {
        address nonOwner = address(3);
        vm.startPrank(nonOwner);

        Forwarder.ForwarderConfig memory newConfig;
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, nonOwner));
        forwarder.updateConfig(abi.encode(newConfig));

        vm.stopPrank();
    }

    function testForwardETH() public {
        // Fund input account
        vm.deal(address(inputAccount), 2 ether);

        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        assertEq(address(outputAccount).balance, 1 ether);
        // Nothing should have been forwarded for the ERC20 token
        assertEq(token.balanceOf(address(outputAccount)), 0);
    }

    function testForwardERC20() public {
        // Fund input account
        token.mint(address(inputAccount), 200);

        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        assertEq(token.balanceOf(address(outputAccount)), 100);
        // Nothing should have been forwarded for the ETH
        assertEq(address(outputAccount).balance, 0);
    }

    function testForwardERC20andETH() public {
        // Fund input account
        vm.deal(address(inputAccount), 2 ether);
        token.mint(address(inputAccount), 200);

        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        assertEq(address(outputAccount).balance, 1 ether);
        assertEq(token.balanceOf(address(outputAccount)), 100);
    }

    function testForwardMaxAmountTwice() public {
        // Fund input account
        vm.deal(address(inputAccount), 2 ether);
        token.mint(address(inputAccount), 200);

        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        assertEq(address(outputAccount).balance, 1 ether);
        assertEq(token.balanceOf(address(outputAccount)), 100);

        // Increase a block
        vm.roll(block.number + 1);

        // Forward again
        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        // Should not forward anything
        assertEq(address(outputAccount).balance, 2 ether);
        assertEq(token.balanceOf(address(outputAccount)), 200);
    }

    function testForwardMaxBalance() public {
        // Fund input account
        vm.deal(address(inputAccount), 0.1 ether);
        token.mint(address(inputAccount), 50);

        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        assertEq(address(outputAccount).balance, 0.1 ether);
        assertEq(token.balanceOf(address(outputAccount)), 50);

        // Increase a block
        vm.roll(block.number + 1);

        // Forward again
        vm.startPrank(processor);
        forwarder.forward();
        vm.stopPrank();

        // Should not forward anything because nothing left
        assertEq(address(outputAccount).balance, 0.1 ether);
        assertEq(token.balanceOf(address(outputAccount)), 50);
    }

    function testCannotForwardBeforeInterval() public {
        vm.startPrank(processor);
        forwarder.forward();

        vm.expectRevert("Block interval not passed");
        forwarder.forward();
        vm.stopPrank();
    }

    function testForwardAfterInterval() public {
        vm.startPrank(processor);
        forwarder.forward();

        // Increase a block
        vm.roll(block.number + 1);

        forwarder.forward();
        vm.stopPrank();
    }

    function testRejectDuplicateTokens() public {
        vm.startPrank(owner);

        Forwarder.ForwardingConfig[] memory duplicateConfigs = new Forwarder.ForwardingConfig[](2);
        duplicateConfigs[0] = Forwarder.ForwardingConfig(address(token), 100);
        duplicateConfigs[1] = Forwarder.ForwardingConfig(address(token), 200);

        Forwarder.ForwarderConfig memory badConfig =
            Forwarder.ForwarderConfig(inputAccount, outputAccount, duplicateConfigs, Forwarder.IntervalType.BLOCKS, 1);

        vm.expectRevert("Duplicate token");
        forwarder.updateConfig(abi.encode(badConfig));

        vm.stopPrank();
    }

    function testRejectNoForwardingConfigs() public {
        vm.startPrank(owner);

        Forwarder.ForwardingConfig[] memory noConfigs = new Forwarder.ForwardingConfig[](0);

        Forwarder.ForwarderConfig memory badConfig =
            Forwarder.ForwarderConfig(inputAccount, outputAccount, noConfigs, Forwarder.IntervalType.BLOCKS, 1);

        vm.expectRevert("No forwarding configs");
        forwarder.updateConfig(abi.encode(badConfig));

        vm.stopPrank();
    }

    function testCannotForwardBeforeTimeInterval() public {
        vm.startPrank(owner);

        // Update config to use time interval
        Forwarder.ForwardingConfig[] memory configs = new Forwarder.ForwardingConfig[](1);
        configs[0] = Forwarder.ForwardingConfig(address(0), 1 ether);

        Forwarder.ForwarderConfig memory timeConfig = Forwarder.ForwarderConfig(
            inputAccount,
            outputAccount,
            configs,
            Forwarder.IntervalType.TIME,
            3600 // 1 hour interval
        );

        forwarder.updateConfig(abi.encode(timeConfig));
        vm.stopPrank();

        // Forward once
        vm.startPrank(processor);
        forwarder.forward();

        // Advance time LESS than interval
        vm.warp(block.timestamp + 1800); // 30 minutes
        vm.expectRevert("Time interval not passed");
        forwarder.forward(); // This should revert
        vm.stopPrank();
    }
}
