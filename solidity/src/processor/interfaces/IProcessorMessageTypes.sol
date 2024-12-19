// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title IProcessorMessageTypes
 * @notice Defines all types used by the Processor contract to decode into the ProcessorMessage
 * @dev Using an interface for types allows them to be imported and used across the project
 *      while keeping the type definitions in one central location
 */
interface IProcessorMessageTypes {
    /**
     * @notice Defines the types of messages that can be processed
     * - Pause: Halts the processor
     * - Resume: Resumes the processor if it's halted
     * - EvictMsgs: Removes messages from a queue at a specific position
     * - SendMsgs: Adds messages to the back of a queue
     * - InsertMsgs: Adds new messages in a queue at a specific position
     */
    enum ProcessorMessageType {
        Pause,
        Resume,
        EvictMsgs,
        SendMsgs,
        InsertMsgs
    }

    /**
     * @notice Defines what queue is going to be used
     * - Medium: Standard priority queue
     * - High: High priority queue
     */
    enum Priority {
        Medium,
        High
    }

    /**
     * @notice Defines types of subroutines that can be executed
     * - Atomic: All functions must succeed or the entire subroutine fails
     * - NonAtomic: Functions are executed one by one and can fail independently
     */
    enum SubroutineType {
        Atomic,
        NonAtomic
    }

    /**
     * @notice Defines how duration values should be interpreted
     * - Height: Duration is measured in block height
     * - Time: Duration is measured in seconds
     */
    enum DurationType {
        Height,
        Time
    }

    /**
     * @notice Defines retry behavior for failed operations
     * - NoRetry: Don't retry on failure
     * - Indefinitely: Keep retrying until success
     * - Amount: Retry a specific number of times
     */
    enum RetryTimesType {
        NoRetry,
        Indefinitely,
        Amount
    }

    /**
     * @notice Main message structure containing the message type and its encoded payload
     * @param messageType The type of the message (from ProcessorMessageType enum)
     * @param message ABI encoded payload specific to the message type
     */
    struct ProcessorMessage {
        ProcessorMessageType messageType;
        bytes message;
    }

    /**
     * @notice Represents a time or block height duration
     * @param durationType Whether this is a time or block height duration
     * @param value The duration value (interpretation depends on durationType, blocks or seconds)
     */
    struct Duration {
        DurationType durationType;
        uint64 value;
    }

    /**
     * @notice Defines retry attempt parameters
     * @param retryType The type of retry behavior to use
     * @param amount Number of retry attempts (only used when retryType is Amount)
     */
    struct RetryTimes {
        RetryTimesType retryType;
        uint64 amount;
    }

    /**
     * @notice Complete retry configuration including timing and attempts
     * @param times Defines how many retry attempts should be made
     * @param interval The duration to wait between retry attempts
     */
    struct RetryLogic {
        RetryTimes times;
        Duration interval;
    }

    /**
     * @notice Represents a function call in an atomic subroutine
     * @param contractAddress The contract to call
     */
    struct AtomicFunction {
        address contractAddress;
    }

    /**
     * @notice Defines a callback to be executed after a Non Atomic function call
     * @param contractAddress The contract to call for the callback (address(0) if no callback)
     * @param callbackMessage The encoded message to send in the callback (empty if no callback)
     */
    struct FunctionCallback {
        address contractAddress;
        bytes callbackMessage;
    }

    /**
     * @notice A subroutine where all functions must succeed
     * @param functions Array of functions to call atomically
     * @param retryLogic Retry configuration for the entire subroutine
     */
    struct AtomicSubroutine {
        AtomicFunction[] functions;
        RetryLogic retryLogic;
    }

    /**
     * @notice Represents a function call in a non-atomic subroutine
     * @param contractAddress The contract to call
     * @param retryLogic Retry configuration for this specific function
     * @param callbackConfirmation Optional callback to execute after the function call to advance to the next non atomic function
     */
    struct NonAtomicFunction {
        address contractAddress;
        RetryLogic retryLogic;
        FunctionCallback callbackConfirmation;
    }

    /**
     * @notice A subroutine where functions can fail independently
     * @param functions Array of functions that can be executed independently
     */
    struct NonAtomicSubroutine {
        NonAtomicFunction[] functions;
    }

    /**
     * @notice Wrapper for either type of subroutine
     * @param subroutineType Whether this is an atomic or non-atomic subroutine
     * @param subroutine The encoded subroutine data (either AtomicSubroutine or NonAtomicSubroutine)
     */
    struct Subroutine {
        SubroutineType subroutineType;
        bytes subroutine;
    }

    /**
     * @notice Message type for inserting new messages into the queue
     * @param executionId Unique identifier for this execution
     * @param queuePosition Position in the queue where messages should be inserted
     * @param priority Processing priority for these messages
     * @param subroutine The subroutine to execute for these messages
     * @param messages Array of encoded messages to process
     */
    struct InsertMsgs {
        uint64 executionId;
        uint64 queuePosition;
        Priority priority;
        Subroutine subroutine;
        bytes[] messages;
    }

    /**
     * @notice Message type for sending messages immediately
     * @param executionId Unique identifier for this execution
     * @param priority Processing priority for these messages
     * @param subroutine The subroutine for these messages
     * @param messages Array of encoded messages to process
     */
    struct SendMsgs {
        uint64 executionId;
        Priority priority;
        Subroutine subroutine;
        bytes[] messages;
    }

    /**
     * @notice Message type for removing messages from the queue
     * @param queuePosition Position in the queue from which to evict messages
     * @param priority What queue to evict messages from
     */
    struct EvictMsgs {
        uint64 queuePosition;
        Priority priority;
    }
}
