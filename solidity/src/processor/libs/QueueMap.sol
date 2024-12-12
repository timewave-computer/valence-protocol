// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title QueueMap
 * @dev Implementation of a queue data structure that uses storage mapping to store arbitrary-sized data
 * Each element in the queue can store variable-length data by splitting it across multiple storage slots
 * Uses namespacing to prevent storage collisions between different queues
 */
library QueueMap {
    /**
     * @dev Queue structure that maintains the state of the queue
     * @param namespace Unique identifier for this queue (prevents storage collisions)
     * @param startIndex Index of the first element (exclusive)
     * @param endIndex Index of the last element (inclusive)
     * Note: Both indices only increase to prevent storage slot reuse
     */
    struct Queue {
        bytes32 namespace;
        uint256 startIndex;
        uint256 endIndex;
    }

    /**
     * @dev Custom errors for gas-efficient error handling
     * IndexOutOfBounds: Attempting to access an index beyond queue bounds
     * InvalidRange: Invalid range parameters provided for operations
     * QueueEmpty: Attempting to pop from an empty queue
     * QueueNotInitialized: Attempting to use a queue before initialization
     */
    error IndexOutOfBounds();
    error InvalidRange();
    error QueueEmpty();
    error QueueNotInitialized();

    /**
     * @dev Creates a new queue with a unique namespace
     * @param _namespace String identifier used to create unique storage space for this queue
     * @return Queue A new queue structure with initialized indices
     * @notice The namespace is hashed to create a unique storage scope
     */
    function createQueue(string memory _namespace) internal pure returns (Queue memory) {
        return Queue({namespace: keccak256(abi.encodePacked(_namespace)), startIndex: 0, endIndex: 0});
    }

    /**
     * @dev Calculates the base storage slot for an element
     * @param self The queue to calculate the slot for
     * @param index The index to calculate the slot for
     * @return bytes32 The storage slot identifier
     * @notice Combines namespace and index to create unique slot for each element
     */
    function getElementBaseSlot(Queue storage self, uint256 index) internal view returns (bytes32) {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }
        return keccak256(abi.encodePacked(self.namespace, index));
    }

    /**
     * @dev Adds an element to the end of the queue
     * @param self The queue to add to
     * @param value The data to store (encoded as bytes)
     * @notice Data is split into 32-byte chunks and stored across multiple slots if needed
     * Storage layout:
     * - Base slot: stores length of data
     * - Subsequent slots: store data chunks (32 bytes each)
     */
    function pushBack(Queue storage self, bytes memory value) internal {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }

        self.endIndex++;
        bytes32 baseSlot = getElementBaseSlot(self, self.endIndex);

        // Store the length of the value in the base slot
        uint256 length = value.length;
        assembly {
            sstore(baseSlot, length)
        }

        // Calculate number of 32-byte slots needed and store data
        uint256 numSlots = (length + 31) / 32;
        for (uint256 i = 0; i < numSlots; i++) {
            bytes32 slot = keccak256(abi.encodePacked(baseSlot, i));
            assembly {
                // Calculate memory position of current chunk:
                // value + 32 (skip length prefix) + (i * 32) (chunk offset)
                let chunk := mload(add(add(value, 32), mul(i, 32)))
                sstore(slot, chunk)
            }
        }
    }

    /**
     * @dev Removes and returns the first element in the queue
     * @param self The queue to remove from
     * @return bytes The removed element's data
     * @notice Clears storage slots after reading to get gas refund
     */
    function popFront(Queue storage self) internal returns (bytes memory) {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }
        if (self.startIndex == self.endIndex) {
            revert QueueEmpty();
        }

        self.startIndex++;
        bytes32 baseSlot = getElementBaseSlot(self, self.startIndex);

        // Read length from base slot
        uint256 length;
        assembly {
            length := sload(baseSlot)
        }

        // Allocate memory for the return value
        bytes memory value = new bytes(length);
        uint256 numSlots = (length + 31) / 32;

        // Read each chunk and clear its storage
        for (uint256 i = 0; i < numSlots; i++) {
            bytes32 slot = keccak256(abi.encodePacked(baseSlot, i));
            assembly {
                let chunk := sload(slot)
                // Store chunk in memory:
                // value + 32 (skip length prefix) + (i * 32) (chunk offset)
                mstore(add(add(value, 32), mul(i, 32)), chunk)
                // Clear storage slot
                sstore(slot, 0)
            }
        }

        // Clear base slot
        assembly {
            sstore(baseSlot, 0)
        }

        return value;
    }

    /**
     * @dev Returns the current number of elements in the queue
     * @param self The queue to measure
     * @return uint256 Number of elements
     */
    function len(Queue storage self) internal view returns (uint256) {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }
        return self.endIndex - self.startIndex;
    }

    /**
     * @dev Checks if the queue is empty
     * @param self The queue to check
     * @return bool True if queue has no elements
     */
    function isEmpty(Queue storage self) internal view returns (bool) {
        return len(self) == 0;
    }

    /**
     * @dev Inserts an element at a specific index in the queue
     * @param self The queue to insert into
     * @param index The position to insert at (0 = front)
     * @param value The data to insert
     * @notice Shifts all elements after the insertion point forward
     * Actual storage position is calculated as startIndex + index + 1
     */
    function insertAt(Queue storage self, uint256 index, bytes memory value) internal {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }

        uint256 length = len(self);
        if (index > length) {
            revert IndexOutOfBounds();
        }

        // Increment endIndex first as we'll need a new slot
        self.endIndex++;

        // Calculate actual storage index (offset by startIndex)
        uint256 actualIndex = self.startIndex + index + 1;

        // Shift elements to make space for the new element
        shiftElementsForward(self, actualIndex);

        // Insert the new element
        storeElement(self, actualIndex, value);
    }

    function shiftElementsForward(Queue storage self, uint256 actualIndex) private {
        for (uint256 i = self.endIndex; i > actualIndex; i--) {
            bytes32 sourceBaseSlot = getElementBaseSlot(self, i - 1);
            bytes32 targetBaseSlot = getElementBaseSlot(self, i);

            // Copy length first
            uint256 elementLength;
            assembly {
                elementLength := sload(sourceBaseSlot)
                sstore(targetBaseSlot, elementLength)
            }

            // Copy all data chunks of the element
            uint256 numSlots = (elementLength + 31) / 32;
            for (uint256 j = 0; j < numSlots; j++) {
                bytes32 sourceSlot = keccak256(abi.encodePacked(sourceBaseSlot, j));
                bytes32 targetSlot = keccak256(abi.encodePacked(targetBaseSlot, j));
                assembly {
                    let chunk := sload(sourceSlot)
                    sstore(targetSlot, chunk)
                }
            }
        }
    }

    function storeElement(Queue storage self, uint256 actualIndex, bytes memory value) private {
        bytes32 insertBaseSlot = getElementBaseSlot(self, actualIndex);
        uint256 insertLength = value.length;
        assembly {
            sstore(insertBaseSlot, insertLength)
        }

        uint256 insertNumSlots = (insertLength + 31) / 32;
        for (uint256 i = 0; i < insertNumSlots; i++) {
            bytes32 slot = keccak256(abi.encodePacked(insertBaseSlot, i));
            assembly {
                let chunk := mload(add(add(value, 32), mul(i, 32)))
                sstore(slot, chunk)
            }
        }
    }

    /**
     * @dev Removes and returns an element at a specific index
     * @param self The queue to remove from
     * @param index The position to remove (0 = front)
     * @return bytes The removed element's data
     * @notice Optimizes removal from front using popFront
     * Shifts all elements after removal point backward
     */
    function removeAt(Queue storage self, uint256 index) internal returns (bytes memory) {
        if (self.namespace == bytes32(0)) {
            revert QueueNotInitialized();
        }

        uint256 length = len(self);
        if (index >= length) {
            revert IndexOutOfBounds();
        }

        // Optimize for removing from front
        if (index == 0) {
            return popFront(self);
        }

        // Calculate actual storage index
        uint256 actualIndex = self.startIndex + index + 1;

        // Retrieve the element to remove
        bytes memory removedValue = retrieveElement(self, actualIndex);

        // Shift elements after the removed one backward
        shiftElementsBackward(self, actualIndex);

        // Clear the last element's slots for gas refund
        clearLastElement(self);

        self.endIndex--;
        return removedValue;
    }

    function retrieveElement(Queue storage self, uint256 actualIndex) private view returns (bytes memory) {
        bytes32 removeBaseSlot = getElementBaseSlot(self, actualIndex);
        uint256 elementLength;
        assembly {
            elementLength := sload(removeBaseSlot)
        }

        bytes memory removedValue = new bytes(elementLength);
        uint256 numSlots = (elementLength + 31) / 32;
        for (uint256 i = 0; i < numSlots; i++) {
            bytes32 slot = keccak256(abi.encodePacked(removeBaseSlot, i));
            assembly {
                let chunk := sload(slot)
                mstore(add(add(removedValue, 32), mul(i, 32)), chunk)
            }
        }

        return removedValue;
    }

    function shiftElementsBackward(Queue storage self, uint256 actualIndex) private {
        for (uint256 i = actualIndex; i < self.endIndex; i++) {
            bytes32 sourceBaseSlot = getElementBaseSlot(self, i + 1);
            bytes32 targetBaseSlot = getElementBaseSlot(self, i);

            // Copy length
            uint256 nextLength;
            assembly {
                nextLength := sload(sourceBaseSlot)
                sstore(targetBaseSlot, nextLength)
            }

            // Copy all chunks
            uint256 nextNumSlots = (nextLength + 31) / 32;
            for (uint256 j = 0; j < nextNumSlots; j++) {
                bytes32 sourceSlot = keccak256(abi.encodePacked(sourceBaseSlot, j));
                bytes32 targetSlot = keccak256(abi.encodePacked(targetBaseSlot, j));
                assembly {
                    let chunk := sload(sourceSlot)
                    sstore(targetSlot, chunk)
                }
            }
        }
    }

    function clearLastElement(Queue storage self) private {
        bytes32 lastBaseSlot = getElementBaseSlot(self, self.endIndex);
        uint256 lastLength;
        assembly {
            lastLength := sload(lastBaseSlot)
            sstore(lastBaseSlot, 0)
        }

        uint256 lastNumSlots = (lastLength + 31) / 32;
        for (uint256 i = 0; i < lastNumSlots; i++) {
            bytes32 slot = keccak256(abi.encodePacked(lastBaseSlot, i));
            assembly {
                sstore(slot, 0)
            }
        }
    }
}
