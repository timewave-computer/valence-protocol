// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/// @title ProcessorMessage Library
/// @notice Library for parsing and handling batched messages and subroutines
/// @dev Handles parsing of complex nested structures from byte arrays
library ProcessorMessage {
    /// @notice Type of interval measurement
    enum IntervalType {
        Blocks,
        Seconds
    }

    /// @notice Contains retry configuration for functions
    /// @param times Specification of retry attempts
    /// @param intervalType Type of interval (blocks or seconds)
    /// @param interval Duration between retry attempts (in blocks or seconds)
    struct RetryLogic {
        RetryTimes times;
        IntervalType intervalType;
        uint64 interval;
    }

    /// @notice Type of retry configuration
    enum RetryTimesType {
        Indefinitely,
        Amount
    }

    /// @notice Specifies how many times to retry
    /// @param retryType Whether to retry indefinitely or a specific amount
    /// @param amount Number of times to retry if type is Amount
    struct RetryTimes {
        RetryTimesType retryType;
        uint64 amount;
    }

    /// @notice Callback configuration for function completion
    /// @param contractAddress Address to receive callback from
    /// @param callbackMessage Expected message to receive in callback
    struct FunctionCallback {
        address contractAddress;
        bytes callbackMessage;
    }

    /// @notice Function configuration for atomic operations
    /// @param contractAddress Target contract address
    struct AtomicFunction {
        address contractAddress;
    }

    /// @notice Function configuration for non-atomic operations
    /// @param contractAddress Target contract address
    /// @param hasRetryLogic Whether retry logic is present
    /// @param retryLogic Retry configuration if present
    /// @param hasCallbackConfirmation Whether callback confirmation is required
    /// @param callbackConfirmation Callback configuration if required
    struct NonAtomicFunction {
        address contractAddress;
        bool hasRetryLogic;
        RetryLogic retryLogic;
        bool hasCallbackConfirmation;
        FunctionCallback callbackConfirmation;
    }

    /// @notice Collection of atomic functions with shared retry logic
    /// @param functions Array of atomic functions
    /// @param hasRetryLogic Whether retry logic is present
    /// @param retryLogic Retry configuration if present
    struct AtomicSubroutine {
        AtomicFunction[] functions;
        bool hasRetryLogic;
        RetryLogic retryLogic;
    }

    /// @notice Collection of non-atomic functions
    /// @param functions Array of non-atomic functions
    struct NonAtomicSubroutine {
        NonAtomicFunction[] functions;
    }

    /// @notice Type of subroutine
    enum SubroutineType {
        Atomic,
        NonAtomic
    }

    /// @notice Container for either atomic or non-atomic subroutine
    /// @param subroutineType Type of contained subroutine
    /// @param atomic Atomic subroutine if type is Atomic
    /// @param nonAtomic Non-atomic subroutine if type is NonAtomic
    struct Subroutine {
        SubroutineType subroutineType;
        AtomicSubroutine atomic;
        NonAtomicSubroutine nonAtomic;
    }

    /// @notice Result of parsing messages from batch
    /// @param messages Array of parsed message bytes
    /// @param offset Next position in byte array after messages
    struct MessagesResult {
        bytes[] messages;
        uint256 offset;
    }

    /// @notice Get type of authorization message that we are executing on the processor
    /// @param message Raw message bytes
    /// @return bytes1 Authorization Message Type value (0x00 for EnqueueMsgs, 0x01 for EvictMsgs, 0x02 for InsertMsgs, 0x03 for Pause and 0x04 for Resume)
    function authorizationMessageType(bytes calldata message) internal pure returns (bytes1) {
        return message[0];
    }

    /// @notice Get queue position for the message batch in the queue (Only used for EvictMsgs and InsertMsgs, for EnqueueMsgs the value will be 0)
    /// @param message Raw message bytes
    /// @return uint64 Queue Position
    function queuePosition(bytes calldata message) internal pure returns (uint64) {
        bytes memory slice = message[1:9];
        return uint64(bytes8(slice));
    }

    /// @notice Get priority from message batch
    /// @param message Raw message bytes
    /// @return bytes1 Priority value (0x00 for Medium, 0x01 for High)
    function priority(bytes calldata message) internal pure returns (bytes1) {
        return message[9];
    }

    /// @notice Get execution ID from message batch
    /// @param message Raw message bytes
    /// @return uint64 Execution ID
    function id(bytes calldata message) internal pure returns (uint64) {
        bytes memory slice = message[10:18];
        return uint64(bytes8(slice));
    }

    /// @notice Get number of messages in batch
    /// @param message Raw message bytes
    /// @return uint64 Number of messages
    function numMessages(bytes calldata message) internal pure returns (uint64) {
        bytes memory slice = message[18:26];
        return uint64(bytes8(slice));
    }

    /// @notice Parse messages from batch
    /// @param message Raw message bytes
    /// @return MessagesResult Parsed messages and next offset
    function messages(bytes calldata message) internal pure returns (MessagesResult memory) {
        uint64 count = numMessages(message);
        bytes[] memory result = new bytes[](count);

        uint256 offset = 26; // Start after header (1 + 8 + 1 + 8 + 8 bytes)

        for (uint64 i = 0; i < count; i++) {
            uint64 length = uint64(bytes8(message[offset:offset + 8]));
            offset += 8;

            result[i] = message[offset:offset + length];
            offset += length;
        }

        return MessagesResult({messages: result, offset: offset});
    }

    /// @notice Parse all subroutines from batch
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return Subroutine[] Array of parsed subroutines
    function parseSubroutines(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (Subroutine[] memory)
    {
        uint64 count = numMessages(message);
        Subroutine[] memory subroutines = new Subroutine[](count);

        uint256 offset = startOffset;
        for (uint64 i = 0; i < count; i++) {
            SubroutineType subroutineType = SubroutineType(uint8(message[offset]));
            offset += 1;

            if (subroutineType == SubroutineType.Atomic) {
                (subroutines[i], offset) = parseAtomicSubroutine(message, offset);
                subroutines[i].subroutineType = SubroutineType.Atomic;
            } else {
                (subroutines[i], offset) = parseNonAtomicSubroutine(message, offset);
                subroutines[i].subroutineType = SubroutineType.NonAtomic;
            }
        }

        return subroutines;
    }

    /// @notice Parse atomic subroutine from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return subroutine Parsed atomic subroutine
    /// @return newOffset Next position in byte array
    function parseAtomicSubroutine(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (Subroutine memory subroutine, uint256 newOffset)
    {
        uint256 offset = startOffset;

        uint64 functionCount = uint64(bytes8(message[offset:offset + 8]));
        offset += 8;

        AtomicFunction[] memory functions = new AtomicFunction[](functionCount);
        for (uint64 i = 0; i < functionCount; i++) {
            (functions[i], offset) = parseAtomicFunction(message, offset);
        }

        bool hasRetryLogic = message[offset] != 0;
        offset += 1;

        RetryLogic memory retryLogic;
        if (hasRetryLogic) {
            (retryLogic, offset) = parseRetryLogic(message, offset);
        }

        subroutine.atomic.functions = functions;
        subroutine.atomic.hasRetryLogic = hasRetryLogic;
        subroutine.atomic.retryLogic = retryLogic;

        return (subroutine, offset);
    }

    /// @notice Parse non-atomic subroutine from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return subroutine Parsed non-atomic subroutine
    /// @return newOffset Next position in byte array
    function parseNonAtomicSubroutine(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (Subroutine memory subroutine, uint256 newOffset)
    {
        uint256 offset = startOffset;

        uint64 functionCount = uint64(bytes8(message[offset:offset + 8]));
        offset += 8;

        NonAtomicFunction[] memory functions = new NonAtomicFunction[](functionCount);
        for (uint64 i = 0; i < functionCount; i++) {
            (functions[i], offset) = parseNonAtomicFunction(message, offset);
        }

        subroutine.nonAtomic.functions = functions;

        return (subroutine, offset);
    }

    /// @notice Parse atomic function from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return func Parsed atomic function
    /// @return newOffset Next position in byte array
    function parseAtomicFunction(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (AtomicFunction memory func, uint256 newOffset)
    {
        uint256 offset = startOffset;

        // Parse 20 bytes for address
        address contractAddress = address(bytes20(message[offset:offset + 20]));
        offset += 20;

        func.contractAddress = contractAddress;
        return (func, offset);
    }

    /// @notice Parse non-atomic function from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return func Parsed non-atomic function
    /// @return newOffset Next position in byte array
    function parseNonAtomicFunction(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (NonAtomicFunction memory func, uint256 newOffset)
    {
        uint256 offset = startOffset;

        // Parse 20 bytes for address
        address contractAddress = address(bytes20(message[offset:offset + 20]));
        offset += 20;
        func.contractAddress = contractAddress;

        func.hasRetryLogic = message[offset] != 0;
        offset += 1;

        if (func.hasRetryLogic) {
            (func.retryLogic, offset) = parseRetryLogic(message, offset);
        }

        func.hasCallbackConfirmation = message[offset] != 0;
        offset += 1;

        if (func.hasCallbackConfirmation) {
            (func.callbackConfirmation, offset) = parseFunctionCallback(message, offset);
        }

        return (func, offset);
    }

    /// @notice Parse retry logic from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return logic Parsed retry logic
    /// @return newOffset Next position in byte array
    function parseRetryLogic(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (RetryLogic memory logic, uint256 newOffset)
    {
        uint256 offset = startOffset;

        // Parse RetryTimes
        RetryTimesType retryType = RetryTimesType(uint8(message[offset]));
        offset += 1;

        uint64 amount = 0;
        if (retryType == RetryTimesType.Amount) {
            amount = uint64(bytes8(message[offset:offset + 8]));
            offset += 8;
        }

        logic.times = RetryTimes({retryType: retryType, amount: amount});

        // Parse interval type
        logic.intervalType = IntervalType(uint8(message[offset]));
        offset += 1;

        // Parse interval
        logic.interval = uint64(bytes8(message[offset:offset + 8]));
        offset += 8;

        return (logic, offset);
    }

    /// @notice Parse function callback from bytes
    /// @param message Raw message bytes
    /// @param startOffset Starting position in byte array
    /// @return callback Parsed function callback
    /// @return newOffset Next position in byte array
    function parseFunctionCallback(bytes calldata message, uint256 startOffset)
        internal
        pure
        returns (FunctionCallback memory callback, uint256 newOffset)
    {
        uint256 offset = startOffset;

        // Parse 20 bytes for address
        address contractAddress = address(bytes20(message[offset:offset + 20]));
        offset += 20;
        callback.contractAddress = contractAddress;

        // Parse callback message
        uint64 messageLength = uint64(bytes8(message[offset:offset + 8]));
        offset += 8;
        callback.callbackMessage = message[offset:offset + messageLength];
        offset += messageLength;

        return (callback, offset);
    }
}
