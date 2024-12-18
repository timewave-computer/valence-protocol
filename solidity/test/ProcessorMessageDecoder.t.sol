// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ProcessorMessageDecoder} from "../src/processor/libs/ProcessorMessageDecoder.sol";
import {IProcessorMessageTypes} from "../src/processor/interfaces/IProcessorMessageTypes.sol";

contract ProcessorMessageDecoderTest is Test {
    // Helper function to create a basic atomic subroutine
    function createAtomicSubroutine() internal pure returns (IProcessorMessageTypes.Subroutine memory) {
        address[] memory addresses = new address[](2);
        addresses[0] = address(0x1);
        addresses[1] = address(0x2);

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
        // Encode a tuple of (messageType, empty bytes) using abi.encode
        bytes memory encoded = abi.encode(
            uint8(IProcessorMessageTypes.ProcessorMessageType.Pause),
            bytes("") // Empty bytes for Pause message
        );

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.Pause));
        assertEq(decoded.message.length, 0);
    }

    function test_DecodeResumeMessage() public pure {
        // Similar to Pause, but with Resume message type
        bytes memory encoded = abi.encode(
            uint8(IProcessorMessageTypes.ProcessorMessageType.Resume),
            bytes("") // Empty bytes for Resume message
        );

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.Resume));
        assertEq(decoded.message.length, 0);
    }

    function test_DecodeEvictMsgsMessage() public pure {
        IProcessorMessageTypes.EvictMsgs memory original =
            IProcessorMessageTypes.EvictMsgs({queuePosition: 42, priority: IProcessorMessageTypes.Priority.High});

        // Encode as a tuple of (messageType, encoded EvictMsgs)
        bytes memory encoded =
            abi.encode(uint8(IProcessorMessageTypes.ProcessorMessageType.EvictMsgs), abi.encode(original));

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.EvictMsgs memory evictMsg =
            abi.decode(decoded.message, (IProcessorMessageTypes.EvictMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.EvictMsgs));
        assertEq(evictMsg.queuePosition, original.queuePosition);
        assertEq(uint8(evictMsg.priority), uint8(original.priority));
    }

    function test_DecodeSendMsgsMessage() public pure {
        bytes[] memory messages = new bytes[](2);
        messages[0] = bytes("msg1");
        messages[1] = bytes("msg2");

        IProcessorMessageTypes.SendMsgs memory original = IProcessorMessageTypes.SendMsgs({
            executionId: 123,
            priority: IProcessorMessageTypes.Priority.Medium,
            subroutine: createAtomicSubroutine(),
            messages: messages
        });

        // Encode as a tuple of (messageType, encoded SendMsgs)
        bytes memory encoded =
            abi.encode(uint8(IProcessorMessageTypes.ProcessorMessageType.SendMsgs), abi.encode(original));

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.SendMsgs memory sendMsg = abi.decode(decoded.message, (IProcessorMessageTypes.SendMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.SendMsgs));
        assertEq(sendMsg.executionId, original.executionId);
        assertEq(uint8(sendMsg.priority), uint8(original.priority));
        assertEq(sendMsg.messages.length, original.messages.length);
        assertEq(string(sendMsg.messages[0]), string(original.messages[0]));
        assertEq(string(sendMsg.messages[1]), string(original.messages[1]));
    }

    function test_DecodeInsertMsgsMessage() public pure {
        bytes[] memory messages = new bytes[](2);
        messages[0] = bytes("msg1");
        messages[1] = bytes("msg2");

        IProcessorMessageTypes.InsertMsgs memory original = IProcessorMessageTypes.InsertMsgs({
            executionId: 456,
            queuePosition: 789,
            priority: IProcessorMessageTypes.Priority.High,
            subroutine: createNonAtomicSubroutine(),
            messages: messages
        });

        // Encode as a tuple of (messageType, encoded InsertMsgs)
        bytes memory encoded =
            abi.encode(uint8(IProcessorMessageTypes.ProcessorMessageType.InsertMsgs), abi.encode(original));

        IProcessorMessageTypes.ProcessorMessage memory decoded = ProcessorMessageDecoder.decode(encoded);
        IProcessorMessageTypes.InsertMsgs memory insertMsg =
            abi.decode(decoded.message, (IProcessorMessageTypes.InsertMsgs));

        assertEq(uint8(decoded.messageType), uint8(IProcessorMessageTypes.ProcessorMessageType.InsertMsgs));
        assertEq(insertMsg.executionId, original.executionId);
        assertEq(insertMsg.queuePosition, original.queuePosition);
        assertEq(uint8(insertMsg.priority), uint8(original.priority));
        assertEq(insertMsg.messages.length, original.messages.length);
        assertEq(string(insertMsg.messages[0]), string(original.messages[0]));
        assertEq(string(insertMsg.messages[1]), string(original.messages[1]));
    }

    function test_InvalidMessageType() public {
        // Create an invalid message type (5)
        bytes memory encoded = abi.encode(uint8(5), bytes(""));

        vm.expectRevert(ProcessorMessageDecoder.InvalidMessageType.selector);
        ProcessorMessageDecoder.decode(encoded);
    }

    function test_DecodeComplexSubroutines() public pure {
        // Test atomic subroutine decoding
        IProcessorMessageTypes.Subroutine memory atomicSub = createAtomicSubroutine();
        IProcessorMessageTypes.AtomicSubroutine memory decodedAtomic =
            abi.decode(atomicSub.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

        assertEq(uint8(atomicSub.subroutineType), uint8(IProcessorMessageTypes.SubroutineType.Atomic));
        assertEq(decodedAtomic.functions.length, 2);
        assertEq(decodedAtomic.functions[0].contractAddress, address(0x1));
        assertEq(uint8(decodedAtomic.retryLogic.times.retryType), uint8(IProcessorMessageTypes.RetryTimesType.Amount));
        assertEq(decodedAtomic.retryLogic.times.amount, 3);
        assertEq(uint8(decodedAtomic.retryLogic.interval.durationType), uint8(IProcessorMessageTypes.DurationType.Time));
        assertEq(decodedAtomic.retryLogic.interval.value, 1800);

        // Test non-atomic subroutine decoding
        IProcessorMessageTypes.Subroutine memory nonAtomicSub = createNonAtomicSubroutine();
        IProcessorMessageTypes.NonAtomicSubroutine memory decodedNonAtomic =
            abi.decode(nonAtomicSub.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

        assertEq(uint8(nonAtomicSub.subroutineType), uint8(IProcessorMessageTypes.SubroutineType.NonAtomic));
        assertEq(decodedNonAtomic.functions.length, 2);
        assertEq(decodedNonAtomic.functions[0].contractAddress, address(0x1));
        assertEq(
            uint8(decodedNonAtomic.functions[0].retryLogic.times.retryType),
            uint8(IProcessorMessageTypes.RetryTimesType.Indefinitely)
        );
        assertEq(decodedNonAtomic.functions[0].callbackConfirmation.contractAddress, address(0x3));
        assertEq(string(decodedNonAtomic.functions[0].callbackConfirmation.callbackMessage), "callback");
    }
}
