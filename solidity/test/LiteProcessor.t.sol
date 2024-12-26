// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title LiteProcessorTest
 * @notice Test suite for the LiteProcessor contract
 * @dev Tests contract deployment, authorization checks, and message handling functionality
 */
import {Test} from "forge-std/src/Test.sol";
import {LiteProcessor} from "../src/processor/LiteProcessor.sol";
import {IProcessorMessageTypes} from "../src/processor/interfaces/IProcessorMessageTypes.sol";
import {ProcessorMessageDecoder} from "../src/processor/libs/ProcessorMessageDecoder.sol";

contract LiteProcessorTest is Test {
    // Main contract instance to be tested
    LiteProcessor public processor;
    // Mock mailbox address that will be authorized to call the processor
    address public constant MAILBOX = address(0x1234);
    // Mock authorization contract address converted to bytes32 for cross-chain representation
    bytes32 public constant AUTH_CONTRACT = bytes32(uint256(uint160(address(0x5678))));

    // Events that we expect the contract to emit
    event MessageReceived(uint32 indexed origin, bytes32 indexed sender, bytes body);
    event ProcessorPaused();
    event ProcessorResumed();

    /// @notice Deploy a fresh instance of the processor before each test
    function setUp() public {
        processor = new LiteProcessor(AUTH_CONTRACT, MAILBOX);
    }

    /// @notice Test that the constructor properly initializes state variables
    function test_Constructor() public view {
        assertEq(address(processor.mailbox()), MAILBOX);
        assertEq(processor.authorizationContract(), AUTH_CONTRACT);
        assertFalse(processor.paused());
    }

    /// @notice Test that constructor reverts when given zero address for mailbox
    function test_Constructor_RevertOnZeroMailbox() public {
        vm.expectRevert(LiteProcessor.InvalidAddressError.selector);
        new LiteProcessor(AUTH_CONTRACT, address(0));
    }

    /// @notice Test that handle() reverts when called by non-mailbox address
    function test_Handle_RevertOnUnauthorizedSender() public {
        bytes memory message = _encodePauseMessage();

        vm.expectRevert(LiteProcessor.UnauthorizedAccessError.selector);
        processor.handle(1, AUTH_CONTRACT, message);
    }

    /// @notice Test that handle() reverts when message is from unauthorized sender
    function test_Handle_RevertOnUnauthorizedContract() public {
        bytes memory message = _encodePauseMessage();
        bytes32 unauthorizedSender = bytes32(uint256(1));

        vm.prank(MAILBOX);
        vm.expectRevert(LiteProcessor.NotAuthorizationContractError.selector);
        processor.handle(1, unauthorizedSender, message);
    }

    /// @notice Test successful pause message handling and event emission
    function test_Handle_PauseMessage() public {
        bytes memory message = _encodePauseMessage();

        vm.prank(MAILBOX);
        // Check for both ProcessorPaused and MessageReceived events
        vm.expectEmit(true, true, false, true);
        emit MessageReceived(1, AUTH_CONTRACT, message);
        vm.expectEmit(true, true, false, true);
        emit ProcessorPaused();

        processor.handle(1, AUTH_CONTRACT, message);
        assertTrue(processor.paused());
    }

    /// @notice Test successful resume message handling and event emission
    function test_Handle_ResumeMessage() public {
        // First pause the processor to test resume functionality
        bytes memory pauseMessage = _encodePauseMessage();
        vm.prank(MAILBOX);
        processor.handle(1, AUTH_CONTRACT, pauseMessage);
        assertTrue(processor.paused());

        // Then test resume message
        bytes memory resumeMessage = _encodeResumeMessage();

        vm.prank(MAILBOX);
        // Check for both ProcessorResumed and MessageReceived events
        vm.expectEmit(true, true, false, true);
        emit MessageReceived(1, AUTH_CONTRACT, resumeMessage);
        vm.expectEmit(true, true, false, true);
        emit ProcessorResumed();

        processor.handle(1, AUTH_CONTRACT, resumeMessage);
        assertFalse(processor.paused());
    }

    /// @notice Test that unsupported operations revert as expected
    function test_Handle_RevertOnUnsupportedOperation() public {
        bytes memory message = _encodeInsertMsgsMessage();

        vm.prank(MAILBOX);
        vm.expectRevert(LiteProcessor.UnsupportedOperationError.selector);
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
