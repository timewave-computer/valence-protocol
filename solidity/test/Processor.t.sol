// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {Processor} from "../src/processor/Processor.sol";

contract ProcessorTest is Test {
    Processor public processor;
    bytes32 public constant MOCK_AUTH_CONTRACT = bytes32(uint256(1));
    address public constant MOCK_MAILBOX = address(0x1234);

    function setUp() public {
        processor = new Processor(MOCK_AUTH_CONTRACT, MOCK_MAILBOX);
    }

    function testConstructorSuccess() public view {
        assertEq(processor.authorizationContract(), MOCK_AUTH_CONTRACT, "Authorization contract not set correctly");
        assertEq(processor.mailbox(), MOCK_MAILBOX, "Mailbox address not set correctly");
        assertEq(processor.paused(), false, "Processor should not be paused initially");
    }

    function testConstructorZeroMailbox() public {
        vm.expectRevert(Processor.InvalidAddressError.selector);
        new Processor(MOCK_AUTH_CONTRACT, address(0));
    }
}
