// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IProcessorMessageTypes} from "../interfaces/IProcessorMessageTypes.sol";

/**
 * @title ProcessorMessageDecoder Library
 * @notice This library handles the decoding of processor messages.
 * The library contains all necessary type definitions and decoding logic for handling
 * various types of messages that can be processed by the processor.
 */
library ProcessorMessageDecoder {
    // Custom error for invalid message types
    error InvalidMessageType();

    /**
     * @notice Decodes a byte array into a ProcessorMessage struct
     * @dev The first byte of the input contains the message type, followed by the encoded message data
     * @param _body The encoded message bytes to decode
     * @return A ProcessorMessage struct containing the decoded message type and data
     */
    function decode(bytes memory _body) internal pure returns (IProcessorMessageTypes.ProcessorMessage memory) {
        // Decode the entire message structure at once
        (uint8 messageTypeRaw, bytes memory message) = abi.decode(_body, (uint8, bytes));

        // Validate the message type
        if (messageTypeRaw > 4) {
            revert InvalidMessageType();
        }

        // Convert the raw uint8 to our enum type
        IProcessorMessageTypes.ProcessorMessageType messageType =
            IProcessorMessageTypes.ProcessorMessageType(messageTypeRaw);

        return IProcessorMessageTypes.ProcessorMessage({messageType: messageType, message: message});
    }

    /**
     * @notice Decodes a subroutine from bytes directly
     */
    function decodeSubroutine(bytes memory _data) internal pure returns (IProcessorMessageTypes.Subroutine memory) {
        return abi.decode(_data, (IProcessorMessageTypes.Subroutine));
    }

    /**
     * @notice Decodes InsertMsgs directly from bytes
     */
    function decodeInsertMsgs(bytes memory _data) internal pure returns (IProcessorMessageTypes.InsertMsgs memory) {
        return abi.decode(_data, (IProcessorMessageTypes.InsertMsgs));
    }

    /**
     * @notice Decodes SendMsgs directly from bytes
     */
    function decodeSendMsgs(bytes memory _data) internal pure returns (IProcessorMessageTypes.SendMsgs memory) {
        return abi.decode(_data, (IProcessorMessageTypes.SendMsgs));
    }

    /**
     * @notice Decodes EvictMsgs directly from bytes
     */
    function decodeEvictMsgs(bytes memory _data) internal pure returns (IProcessorMessageTypes.EvictMsgs memory) {
        return abi.decode(_data, (IProcessorMessageTypes.EvictMsgs));
    }

    /**
     * @notice Decodes the message payload based on message type
     */
    function decodeMessagePayload(IProcessorMessageTypes.ProcessorMessage memory _message)
        internal
        pure
        returns (bytes memory)
    {
        if (_message.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs) {
            return abi.encode(decodeInsertMsgs(_message.message));
        } else if (_message.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            return abi.encode(decodeSendMsgs(_message.message));
        } else if (_message.messageType == IProcessorMessageTypes.ProcessorMessageType.EvictMsgs) {
            return abi.encode(decodeEvictMsgs(_message.message));
        }
        return "";
    }
}
