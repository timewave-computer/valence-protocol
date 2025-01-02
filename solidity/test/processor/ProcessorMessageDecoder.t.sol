// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {ProcessorMessageDecoder} from "../../src/processor/libs/ProcessorMessageDecoder.sol";
import {IProcessorMessageTypes} from "../../src/processor/interfaces/IProcessorMessageTypes.sol";

contract ProcessorMessageDecoderTest is Test {
    // Helper function to create a basic atomic subroutine
    function createAtomicSubroutine() internal pure returns (IProcessorMessageTypes.Subroutine memory) {
        // Create some addresses that the atomic functions will call, any address will do
        address[] memory addresses = new address[](2);
        addresses[0] = address(0x1);
        addresses[1] = address(0x2);

        // Create the atomic functions using the addresses created before
        IProcessorMessageTypes.AtomicFunction[] memory functions = new IProcessorMessageTypes.AtomicFunction[](2);
        functions[0] = IProcessorMessageTypes.AtomicFunction(addresses[0]);
        functions[1] = IProcessorMessageTypes.AtomicFunction(addresses[1]);

        IProcessorMessageTypes.RetryTimes memory times =
            IProcessorMessageTypes.RetryTimes({retryType: IProcessorMessageTypes.RetryTimesType.Amount, amount: 3});

        IProcessorMessageTypes.Duration memory duration = IProcessorMessageTypes.Duration({
            durationType: IProcessorMessageTypes.DurationType.Time,
            value: 1800 // 30 minutes
        });

        IProcessorMessageTypes.RetryLogic memory retryLogic =
            IProcessorMessageTypes.RetryLogic({times: times, interval: duration});

        IProcessorMessageTypes.AtomicSubroutine memory atomicSub =
            IProcessorMessageTypes.AtomicSubroutine({functions: functions, retryLogic: retryLogic});

        return IProcessorMessageTypes.Subroutine({
            subroutineType: IProcessorMessageTypes.SubroutineType.Atomic,
            subroutine: abi.encode(atomicSub)
        });
    }

    // Helper function to create a basic non-atomic subroutine
    function createNonAtomicSubroutine() internal pure returns (IProcessorMessageTypes.Subroutine memory) {
        IProcessorMessageTypes.NonAtomicFunction[] memory functions = new IProcessorMessageTypes.NonAtomicFunction[](2);

        IProcessorMessageTypes.RetryLogic memory retryLogic = IProcessorMessageTypes.RetryLogic({
            times: IProcessorMessageTypes.RetryTimes({
                retryType: IProcessorMessageTypes.RetryTimesType.Indefinitely,
                amount: 0
            }),
            interval: IProcessorMessageTypes.Duration({durationType: IProcessorMessageTypes.DurationType.Height, value: 10})
        });

        IProcessorMessageTypes.FunctionCallback memory callback =
            IProcessorMessageTypes.FunctionCallback({contractAddress: address(0x3), callbackMessage: bytes("callback")});

        functions[0] = IProcessorMessageTypes.NonAtomicFunction({
            contractAddress: address(0x1),
            retryLogic: retryLogic,
            callbackConfirmation: callback
        });

        functions[1] = IProcessorMessageTypes.NonAtomicFunction({
            contractAddress: address(0x2),
            retryLogic: retryLogic,
            callbackConfirmation: callback
        });

        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSub =
            IProcessorMessageTypes.NonAtomicSubroutine({functions: functions});

        return IProcessorMessageTypes.Subroutine({
            subroutineType: IProcessorMessageTypes.SubroutineType.NonAtomic,
            subroutine: abi.encode(nonAtomicSub)
        });
    }

    function test_DecodePauseMessage() public pure {
        // Create ProcessorMessage bytes similar to how Rust encodes it
        IProcessorMessageTypes.ProcessorMessage memory original = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Pause,
            message: "" // Empty bytes for Pause
        });

        // Encode the entire struct
        bytes memory encoded = abi.encode(original);

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.Pause));
        assertEq(decoded.message.length, 0);
    }

    function test_DecodeResumeMessage() public pure {
        // Create ProcessorMessage bytes similar to how Rust encodes it
        IProcessorMessageTypes.ProcessorMessage memory original = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Resume,
            message: "" // Empty bytes for Resume
        });

        // Encode the entire struct
        bytes memory encoded = abi.encode(original);

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.Resume));
        assertEq(decoded.message.length, 0);
    }

    function test_DecodeEvictMsgsMessage() public pure {
        // First create the EvictMsgs struct
        IProcessorMessageTypes.EvictMsgs memory evictMsgs =
            IProcessorMessageTypes.EvictMsgs({queuePosition: 42, priority: IProcessorMessageTypes.Priority.High});

        // Create the ProcessorMessage with EvictMsgs encoded as its message
        IProcessorMessageTypes.ProcessorMessage memory original = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.EvictMsgs,
            message: abi.encode(evictMsgs)
        });

        // Encode the entire ProcessorMessage struct
        bytes memory encoded = abi.encode(original);

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.EvictMsgs memory evictMsg =
            abi.decode(decoded.message, (IProcessorMessageTypes.EvictMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.EvictMsgs));
        assertEq(evictMsg.queuePosition, evictMsgs.queuePosition);
        assertEq(uint8(evictMsg.priority), uint8(evictMsgs.priority));
    }

    function test_DecodeSendMsgsMessage() public pure {
        bytes[] memory messages = new bytes[](2);
        messages[0] = bytes("msg1");
        messages[1] = bytes("msg2");

        // Create the SendMsgs struct
        IProcessorMessageTypes.SendMsgs memory sendMsgs = IProcessorMessageTypes.SendMsgs({
            executionId: 123,
            priority: IProcessorMessageTypes.Priority.Medium,
            subroutine: createAtomicSubroutine(),
            messages: messages
        });

        // Create the ProcessorMessage with SendMsgs encoded as its message
        IProcessorMessageTypes.ProcessorMessage memory original = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.SendMsgs,
            message: abi.encode(sendMsgs)
        });

        // Encode the entire ProcessorMessage struct
        bytes memory encoded = abi.encode(original);

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.SendMsgs memory decodedSendMsg =
            abi.decode(decoded.message, (IProcessorMessageTypes.SendMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.SendMsgs));
        assertEq(decodedSendMsg.executionId, sendMsgs.executionId);
        assertEq(uint8(decodedSendMsg.priority), uint8(sendMsgs.priority));
        assertEq(decodedSendMsg.messages.length, sendMsgs.messages.length);
        assertEq(string(decodedSendMsg.messages[0]), string(sendMsgs.messages[0]));
        assertEq(string(decodedSendMsg.messages[1]), string(sendMsgs.messages[1]));
    }

    function test_DecodeInsertMsgsMessage() public pure {
        bytes[] memory messages = new bytes[](2);
        messages[0] = bytes("msg1");
        messages[1] = bytes("msg2");

        // Create the InsertMsgs struct
        IProcessorMessageTypes.InsertMsgs memory insertMsgs = IProcessorMessageTypes.InsertMsgs({
            executionId: 456,
            queuePosition: 789,
            priority: IProcessorMessageTypes.Priority.High,
            subroutine: createNonAtomicSubroutine(),
            messages: messages
        });

        // Create the ProcessorMessage with InsertMsgs encoded as its message
        IProcessorMessageTypes.ProcessorMessage memory original = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.InsertMsgs,
            message: abi.encode(insertMsgs)
        });

        // Encode the entire ProcessorMessage struct
        bytes memory encoded = abi.encode(original);

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.InsertMsgs memory decodedInsertMsg =
            abi.decode(decoded.message, (IProcessorMessageTypes.InsertMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.InsertMsgs));
        assertEq(decodedInsertMsg.executionId, insertMsgs.executionId);
        assertEq(decodedInsertMsg.queuePosition, insertMsgs.queuePosition);
        assertEq(uint8(decodedInsertMsg.priority), uint8(insertMsgs.priority));
        assertEq(decodedInsertMsg.messages.length, insertMsgs.messages.length);
        assertEq(string(decodedInsertMsg.messages[0]), string(insertMsgs.messages[0]));
        assertEq(string(decodedInsertMsg.messages[1]), string(insertMsgs.messages[1]));
    }

    function test_DecodePauseMessageFromRust() public pure {
        // These are the exact bytes produced by the CosmWasm contract
        bytes memory encodedFromRust =
            hex"0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000";

        // Decode using the same decoder as other tests
        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encodedFromRust);

        // Verify the results match what we expect
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.Pause));
        assertEq(decoded.message.length, 0);
    }

    function test_DecodeSendMsgsFromRust() public pure {
        // These are the exact bytes produced by the CosmWasm contract for the SendMsgs test
        bytes memory encodedFromRust =
            hex"00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000002e0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000001e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000064000000000000000000000000000000000000000000000000000000000000000100000000000000000000000001010101010101010101010101010101010101010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000004d264e05e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004d264e05e00000000000000000000000000000000000000000000000000000000";

        // Step 1: Decode the main ProcessorMessage structure
        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encodedFromRust);

        // Verify message type is SendMsgs
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.SendMsgs));

        // Step 2: Decode the SendMsgs payload
        IProcessorMessageTypes.SendMsgs memory sendMsgs = ProcessorMessageDecoder.decodeSendMsgs(decoded.message);

        // Step 3: Verify basic SendMsgs fields
        assertEq(sendMsgs.executionId, 1);
        assertEq(uint8(sendMsgs.priority), uint8(IProcessorMessageTypes.Priority.Medium));
        assertEq(sendMsgs.messages.length, 2);

        // Step 4: Verify subroutine structure
        assertEq(uint8(sendMsgs.subroutine.subroutineType), uint8(IProcessorMessageTypes.SubroutineType.Atomic));

        // Step 5: Decode and verify atomic subroutine
        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

        // Verify atomic functions array
        assertEq(atomicSubroutine.functions.length, 1);
        // Create the expected address - all bytes are 0x01
        address expectedAddress = address(0x0101010101010101010101010101010101010101);
        assertEq(atomicSubroutine.functions[0].contractAddress, expectedAddress);

        // Verify retry logic
        assertEq(
            uint8(atomicSubroutine.retryLogic.times.retryType), uint8(IProcessorMessageTypes.RetryTimesType.Amount)
        );
        assertEq(atomicSubroutine.retryLogic.times.amount, 3);
        assertEq(
            uint8(atomicSubroutine.retryLogic.interval.durationType), uint8(IProcessorMessageTypes.DurationType.Height)
        );
        assertEq(atomicSubroutine.retryLogic.interval.value, 100);

        // Verify messages array contains data
        for (uint256 i = 0; i < sendMsgs.messages.length; i++) {
            assert(sendMsgs.messages[i].length > 0);
        }
    }
}
