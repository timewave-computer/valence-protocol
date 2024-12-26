// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {Processor} from "../src/processor/Processor.sol";
import {ProcessorErrors} from "../src/processor/libs/ProcessorErrors.sol";

contract ProcessorTest is Test {
    // Main contract instance to be tested
    Processor public processor;
    // Mock mailbox address that will be authorized to call the processor
    address public constant MAILBOX = address(0x1234);
    // Mock authorization contract address converted to bytes32 for cross-chain representation
    bytes32 public constant AUTH_CONTRACT = bytes32(uint256(uint160(address(0x5678))));
    // Domain ID of the origin domain
    uint32 public constant ORIGIN_DOMAIN = 1;

    /// @notice Deploy a fresh instance of the processor before each test
    function setUp() public {
        processor = new Processor(AUTH_CONTRACT, MAILBOX, ORIGIN_DOMAIN);
    }

    /// @notice Test that the constructor properly initializes state variables
    function testConstructor() public view {
        assertEq(address(processor.mailbox()), MAILBOX);
        assertEq(processor.authorizationContract(), AUTH_CONTRACT);
        assertFalse(processor.paused());
    }

    /// @notice Test that constructor reverts when given zero address for mailbox
    function testConstructorRevertOnZeroMailbox() public {
        vm.expectRevert(ProcessorErrors.InvalidAddress.selector);
        new Processor(AUTH_CONTRACT, address(0), ORIGIN_DOMAIN);
    }
}
