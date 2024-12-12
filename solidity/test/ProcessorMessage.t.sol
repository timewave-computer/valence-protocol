// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ProcessorMessage} from "../src/processor/libs/ProcessorMessage.sol";

contract ProcessorMessageTest is Test {
    // Test data setup
    bytes1 private constant ENQUEUE_TYPE = 0x00;
    bytes1 private constant EVICT_TYPE = 0x01;
    bytes1 private constant INSERT_TYPE = 0x02;
    bytes1 private constant PAUSE_TYPE = 0x03;
    bytes1 private constant RESUME_TYPE = 0x04;

    bytes1 private constant MEDIUM_PRIORITY = 0x00;
    bytes1 private constant HIGH_PRIORITY = 0x01;

    function testHeaderParsing() public {
        // Debug current message format
        bytes memory message = bytes.concat(
            ENQUEUE_TYPE, // 1 byte - index 0
            bytes8(uint64(123)), // 8 bytes - indices 1-8
            MEDIUM_PRIORITY, // 1 byte - index 9
            bytes8(uint64(456)), // 8 bytes - indices 10-17
            bytes8(uint64(2)) // 8 bytes - indices 18-25
        );

        // Make sure we have enough bytes
        assert(message.length >= 26);

        // Call validation through external function to convert to calldata
        this.validateHeader(message);
    }

    function validateHeader(bytes calldata message) external pure {
        // First test just the auth type to isolate the issue
        bytes1 authType = ProcessorMessage.authorizationMessageType(message);
        assertEq(authType, ENQUEUE_TYPE, "Auth type mismatch");

        // Then test queue position
        uint64 queuePos = ProcessorMessage.queuePosition(message);
        assertEq(queuePos, 123, "Queue position mismatch");

        // Continue with remaining checks
        bytes1 prio = ProcessorMessage.priority(message);
        uint64 execId = ProcessorMessage.id(message);
        uint64 numMsgs = ProcessorMessage.numMessages(message);

        assertEq(prio, MEDIUM_PRIORITY);
        assertEq(execId, 456);
        assertEq(numMsgs, 2);
    }

    function testMessageParsing() public {
        // Create two sample messages
        bytes memory message1 = hex"0123456789";
        bytes memory message2 = hex"abcdef";

        // Create full message with header and messages
        bytes memory fullMessage = abi.encodePacked(
            ENQUEUE_TYPE, // Authorization type
            bytes8(uint64(0)), // Queue position
            MEDIUM_PRIORITY, // Priority
            bytes8(uint64(1)), // Execution ID
            bytes8(uint64(2)), // Number of messages
            bytes8(uint64(message1.length)), // Length of first message
            message1, // First message
            bytes8(uint64(message2.length)), // Length of second message
            message2 // Second message
        );

        // Convert to calldata by passing through external function
        this.validateMessages(fullMessage);
    }

    function validateMessages(bytes calldata fullMessage) external pure {
        ProcessorMessage.MessagesResult memory result = ProcessorMessage.messages(fullMessage);

        assertEq(result.messages.length, 2);
        assertEq(result.messages[0], hex"0123456789");
        assertEq(result.messages[1], hex"abcdef");
    }

    function testAtomicSubroutineParsing() public {
        address testAddress = address(0x1234567890123456789012345678901234567890);

        // Create atomic subroutine data
        bytes memory subroutineData = abi.encodePacked(
            uint8(ProcessorMessage.SubroutineType.Atomic), // Subroutine type
            bytes8(uint64(1)), // Number of functions
            bytes20(testAddress), // Contract address
            uint8(1), // Has retry logic
            uint8(ProcessorMessage.RetryTimesType.Amount), // Retry type
            bytes8(uint64(5)), // Retry amount
            uint8(ProcessorMessage.IntervalType.Blocks), // Interval type
            bytes8(uint64(10)) // Interval
        );

        // Create full message with header
        bytes memory fullMessage = abi.encodePacked(
            ENQUEUE_TYPE,
            bytes8(uint64(0)),
            MEDIUM_PRIORITY,
            bytes8(uint64(1)),
            bytes8(uint64(1)),
            bytes8(uint64(20)), // Length of a dummy message
            bytes20(0x0), // Dummy message
            subroutineData
        );

        // Convert to calldata by passing through external function
        this.validateAtomicSubroutine(fullMessage, testAddress);
    }

    function validateAtomicSubroutine(bytes calldata fullMessage, address testAddress) external pure {
        ProcessorMessage.MessagesResult memory messagesResult = ProcessorMessage.messages(fullMessage);
        ProcessorMessage.Subroutine[] memory subroutines =
            ProcessorMessage.parseSubroutines(fullMessage, messagesResult.offset);

        assertEq(uint8(subroutines[0].subroutineType), uint8(ProcessorMessage.SubroutineType.Atomic));
        assertEq(subroutines[0].atomic.functions.length, 1);
        assertEq(subroutines[0].atomic.functions[0].contractAddress, testAddress);
        assertTrue(subroutines[0].atomic.hasRetryLogic);
        assertEq(uint8(subroutines[0].atomic.retryLogic.times.retryType), uint8(ProcessorMessage.RetryTimesType.Amount));
        assertEq(subroutines[0].atomic.retryLogic.times.amount, 5);
        assertEq(uint8(subroutines[0].atomic.retryLogic.intervalType), uint8(ProcessorMessage.IntervalType.Blocks));
        assertEq(subroutines[0].atomic.retryLogic.interval, 10);
    }

    function testNonAtomicSubroutineParsing() public {
        address testAddress = address(0x1234567890123456789012345678901234567890);
        address callbackAddress = address(0x2234567890123456789012345678901234567890);
        bytes memory callbackMessage = hex"deadbeef";

        // Create non-atomic subroutine data
        bytes memory subroutineData = abi.encodePacked(
            uint8(ProcessorMessage.SubroutineType.NonAtomic), // Subroutine type
            bytes8(uint64(1)), // Number of functions
            bytes20(testAddress), // Contract address
            uint8(1), // Has retry logic
            uint8(ProcessorMessage.RetryTimesType.Amount), // Retry type
            bytes8(uint64(5)), // Retry amount
            uint8(ProcessorMessage.IntervalType.Seconds), // Interval type
            bytes8(uint64(10)), // Interval
            uint8(1), // Has callback
            bytes20(callbackAddress), // Callback address
            bytes8(uint64(callbackMessage.length)), // Callback message length
            callbackMessage // Callback message
        );

        // Create full message with header
        bytes memory fullMessage = abi.encodePacked(
            ENQUEUE_TYPE,
            bytes8(uint64(0)),
            MEDIUM_PRIORITY,
            bytes8(uint64(1)),
            bytes8(uint64(1)),
            bytes8(uint64(20)), // Length of a dummy message
            bytes20(0x0), // Dummy message
            subroutineData
        );

        this.validateNonAtomicSubroutine(fullMessage, testAddress, callbackAddress, callbackMessage);
    }

    function validateNonAtomicSubroutine(
        bytes calldata fullMessage,
        address testAddress,
        address callbackAddress,
        bytes memory callbackMessage
    ) external pure {
        ProcessorMessage.MessagesResult memory messagesResult = ProcessorMessage.messages(fullMessage);
        ProcessorMessage.Subroutine[] memory subroutines =
            ProcessorMessage.parseSubroutines(fullMessage, messagesResult.offset);

        assertEq(uint8(subroutines[0].subroutineType), uint8(ProcessorMessage.SubroutineType.NonAtomic));
        assertEq(subroutines[0].nonAtomic.functions.length, 1);
        assertEq(subroutines[0].nonAtomic.functions[0].contractAddress, testAddress);
        assertTrue(subroutines[0].nonAtomic.functions[0].hasRetryLogic);
        assertEq(
            uint8(subroutines[0].nonAtomic.functions[0].retryLogic.times.retryType),
            uint8(ProcessorMessage.RetryTimesType.Amount)
        );
        assertEq(subroutines[0].nonAtomic.functions[0].retryLogic.times.amount, 5);
        assertEq(
            uint8(subroutines[0].nonAtomic.functions[0].retryLogic.intervalType),
            uint8(ProcessorMessage.IntervalType.Seconds)
        );
        assertEq(subroutines[0].nonAtomic.functions[0].retryLogic.interval, 10);
        assertTrue(subroutines[0].nonAtomic.functions[0].hasCallbackConfirmation);
        assertEq(subroutines[0].nonAtomic.functions[0].callbackConfirmation.contractAddress, callbackAddress);
        assertEq(subroutines[0].nonAtomic.functions[0].callbackConfirmation.callbackMessage, callbackMessage);
    }

    function testIndefiniteRetry() public {
        address testAddress = address(0x1234567890123456789012345678901234567890);

        // Create atomic subroutine data with indefinite retry
        bytes memory subroutineData = abi.encodePacked(
            uint8(ProcessorMessage.SubroutineType.Atomic), // Subroutine type
            bytes8(uint64(1)), // Number of functions
            bytes20(testAddress), // Contract address
            uint8(1), // Has retry logic
            uint8(ProcessorMessage.RetryTimesType.Indefinitely), // Retry type
            uint8(ProcessorMessage.IntervalType.Blocks), // Interval type
            bytes8(uint64(10)) // Interval
        );

        // Create full message with header
        bytes memory fullMessage = abi.encodePacked(
            ENQUEUE_TYPE,
            bytes8(uint64(0)),
            MEDIUM_PRIORITY,
            bytes8(uint64(1)),
            bytes8(uint64(1)),
            bytes8(uint64(20)), // Length of a dummy message
            bytes20(0x0), // Dummy message
            subroutineData
        );

        this.validateIndefiniteRetry(fullMessage);
    }

    function validateIndefiniteRetry(bytes calldata fullMessage) external pure {
        ProcessorMessage.MessagesResult memory messagesResult = ProcessorMessage.messages(fullMessage);
        ProcessorMessage.Subroutine[] memory subroutines =
            ProcessorMessage.parseSubroutines(fullMessage, messagesResult.offset);

        assertTrue(subroutines[0].atomic.hasRetryLogic);
        assertEq(
            uint8(subroutines[0].atomic.retryLogic.times.retryType), uint8(ProcessorMessage.RetryTimesType.Indefinitely)
        );
        assertEq(subroutines[0].atomic.retryLogic.times.amount, 0);
    }

    function testMultipleAtomicFunctions() public {
        address testAddress1 = address(0x1234567890123456789012345678901234567890);
        address testAddress2 = address(0x2234567890123456789012345678901234567890);

        // Create atomic subroutine data with multiple functions
        bytes memory subroutineData = abi.encodePacked(
            uint8(ProcessorMessage.SubroutineType.Atomic), // Subroutine type
            bytes8(uint64(2)), // Number of functions
            bytes20(testAddress1), // First contract address
            bytes20(testAddress2), // Second contract address
            uint8(0) // No retry logic
        );

        // Create full message
        bytes memory fullMessage = abi.encodePacked(
            ENQUEUE_TYPE,
            bytes8(uint64(0)),
            MEDIUM_PRIORITY,
            bytes8(uint64(1)),
            bytes8(uint64(1)),
            bytes8(uint64(20)), // Length of a dummy message
            bytes20(0x0), // Dummy message
            subroutineData
        );

        this.validateMultipleAtomicFunctions(fullMessage, testAddress1, testAddress2);
    }

    function validateMultipleAtomicFunctions(bytes calldata fullMessage, address testAddress1, address testAddress2)
        external
        pure
    {
        ProcessorMessage.MessagesResult memory messagesResult = ProcessorMessage.messages(fullMessage);
        ProcessorMessage.Subroutine[] memory subroutines =
            ProcessorMessage.parseSubroutines(fullMessage, messagesResult.offset);

        assertEq(subroutines[0].atomic.functions.length, 2);
        assertEq(subroutines[0].atomic.functions[0].contractAddress, testAddress1);
        assertEq(subroutines[0].atomic.functions[1].contractAddress, testAddress2);
        assertFalse(subroutines[0].atomic.hasRetryLogic);
    }

    function testFailInvalidAuthorizationType() public {
        bytes memory message = abi.encodePacked(
            bytes1(0x05), // Invalid authorization type
            bytes8(uint64(0)), // Queue position
            MEDIUM_PRIORITY, // Priority
            bytes8(uint64(1)), // Execution ID
            bytes8(uint64(0)) // Number of messages
        );

        vm.expectRevert(); // Expect the next call to revert
        this.validateInvalidAuthType(message);
    }

    function validateInvalidAuthType(bytes calldata message) external pure {
        ProcessorMessage.authorizationMessageType(message);
    }
}
