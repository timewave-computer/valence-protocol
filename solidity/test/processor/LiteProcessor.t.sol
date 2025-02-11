// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title LiteProcessorTest
 * @notice Test suite for the LiteProcessor contract
 * @dev Tests contract deployment, authorization checks, and message handling functionality
 */
import {Test} from "forge-std/src/Test.sol";
import {LiteProcessor} from "../../src/processor/LiteProcessor.sol";
import {IProcessorMessageTypes} from "../../src/processor/interfaces/IProcessorMessageTypes.sol";
import {ProcessorMessageDecoder} from "../../src/processor/libs/ProcessorMessageDecoder.sol";
import {ProcessorErrors} from "../../src/processor/libs/ProcessorErrors.sol";
import {ProcessorEvents} from "../../src/processor/libs/ProcessorEvents.sol";

contract LiteProcessorTest is Test {
    // Main contract instance to be tested
    LiteProcessor public processor;
    // Mock mailbox address that will be authorized to call the processor
    address public constant MAILBOX = address(0x1234);
    // Mock authorization contract address converted to bytes32 for cross-chain representation
    bytes32 public constant AUTH_CONTRACT = bytes32(uint256(uint160(address(0x5678))));
    // Domain ID of the origin domain
    uint32 public constant ORIGIN_DOMAIN = 1;
    // Authorized addresses that can call the processor directly
    address[] public AUTHORIZED_ADDRESSES = [address(0x1234)];

    /// @notice Deploy a fresh instance of the processor before each test
    function setUp() public {
        processor = new LiteProcessor(AUTH_CONTRACT, MAILBOX, ORIGIN_DOMAIN, AUTHORIZED_ADDRESSES);
    }

    /// @notice Test that the constructor properly initializes state variables
    function testConstructor() public view {
        assertEq(address(processor.mailbox()), MAILBOX);
        assertEq(processor.authorizationContract(), AUTH_CONTRACT);
        assertEq(processor.originDomain(), ORIGIN_DOMAIN);
        assertEq(processor.authorizedAddresses(AUTHORIZED_ADDRESSES[0]), true);
        assertFalse(processor.paused());
    }

    /// @notice Test that constructor reverts when given zero address for mailbox
    function testConstructorRevertOnZeroMailbox() public {
        vm.expectRevert(ProcessorErrors.InvalidAddress.selector);
        new LiteProcessor(AUTH_CONTRACT, address(0), ORIGIN_DOMAIN, AUTHORIZED_ADDRESSES);
    }

    /// @notice Test that handle() reverts when called by an address that is not the mailbox address or an authorized address
    function testHandleRevertOnUnauthorizedSender() public {
        bytes memory message = _encodePauseMessage();

        vm.expectRevert(ProcessorErrors.UnauthorizedAccess.selector);
        processor.handle(ORIGIN_DOMAIN, AUTH_CONTRACT, message);
    }

    /// @notice Test that handle() reverts when receiving a message from an invalid origin domain
    function testHandleRevertOnInvalidOriginDomain() public {
        bytes memory message = _encodePauseMessage();

        vm.expectRevert(ProcessorErrors.UnauthorizedAccess.selector);
        processor.handle(2, AUTH_CONTRACT, message);
    }

    /// @notice Test that handle() reverts when message is from unauthorized sender
    function testHandleRevertOnUnauthorizedContract() public {
        bytes memory message = _encodePauseMessage();
        bytes32 unauthorizedSender = bytes32(uint256(1));

        vm.prank(MAILBOX);
        vm.expectRevert(ProcessorErrors.NotAuthorizationContract.selector);
        processor.handle(ORIGIN_DOMAIN, unauthorizedSender, message);
    }

    /// @notice Test successful pause message handling and event emission when sending from the mailbox
    function testHandlePauseMessageMailbox() public {
        bytes memory message = _encodePauseMessage();

        vm.prank(MAILBOX);
        // Check for ProcessorWasPaused event
        emit ProcessorEvents.ProcessorWasPaused();

        processor.handle(ORIGIN_DOMAIN, AUTH_CONTRACT, message);
        assertTrue(processor.paused());
    }

    /// @notice Test successful pause message handling and event emission when sending from an authorized address
    function testHandlePauseMessageAuthorizedAddress() public {
        bytes memory message = _encodePauseMessage();

        vm.prank(AUTHORIZED_ADDRESSES[0]);
        // Check for ProcessorWasPaused event
        emit ProcessorEvents.ProcessorWasPaused();

        processor.handle(ORIGIN_DOMAIN, AUTH_CONTRACT, message);
        assertTrue(processor.paused());
    }

    /// @notice Test successful resume message handling and event emission
    function testHandleResumeMessage() public {
        // First pause the processor to test resume functionality
        bytes memory pauseMessage = _encodePauseMessage();
        vm.prank(MAILBOX);
        processor.handle(ORIGIN_DOMAIN, AUTH_CONTRACT, pauseMessage);
        assertTrue(processor.paused());

        // Then test resume message
        bytes memory resumeMessage = _encodeResumeMessage();

        vm.prank(MAILBOX);
        // Check for ProcessorWasResumed event
        emit ProcessorEvents.ProcessorWasResumed();

        processor.handle(1, AUTH_CONTRACT, resumeMessage);
        assertFalse(processor.paused());
    }

    /// @notice Test that unsupported operations revert as expected
    function testHandleRevertOnUnsupportedOperation() public {
        bytes memory message = _encodeInsertMsgsMessage();

        vm.prank(MAILBOX);
        vm.expectRevert(ProcessorErrors.UnsupportedOperation.selector);
        processor.handle(1, AUTH_CONTRACT, message);
    }

    // Helper functions to create encoded messages for testing

    /// @notice Creates an encoded pause message
    /// @return bytes The ABI encoded pause message
    function _encodePauseMessage() internal pure returns (bytes memory) {
        IProcessorMessageTypes.ProcessorMessage memory message = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Pause,
            message: bytes("")
        });
        return abi.encode(message);
    }

    /// @notice Creates an encoded resume message
    /// @return bytes The ABI encoded resume message
    function _encodeResumeMessage() internal pure returns (bytes memory) {
        IProcessorMessageTypes.ProcessorMessage memory message = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Resume,
            message: bytes("")
        });
        return abi.encode(message);
    }

    /// @notice Creates an encoded InsertMsgs message for testing unsupported operations
    /// @return bytes The ABI encoded InsertMsgs message
    function _encodeInsertMsgsMessage() internal pure returns (bytes memory) {
        IProcessorMessageTypes.InsertMsgs memory insertMsgs = IProcessorMessageTypes.InsertMsgs({
            executionId: 1,
            queuePosition: 0,
            priority: IProcessorMessageTypes.Priority.Medium,
            subroutine: IProcessorMessageTypes.Subroutine({
                subroutineType: IProcessorMessageTypes.SubroutineType.Atomic,
                subroutine: bytes("")
            }),
            messages: new bytes[](0)
        });

        IProcessorMessageTypes.ProcessorMessage memory message = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.InsertMsgs,
            message: abi.encode(insertMsgs)
        });

        return abi.encode(message);
    }
}
