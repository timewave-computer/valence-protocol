// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {QueueMap} from "../../src/processor/libs/QueueMap.sol";

contract QueueMapTest is Test {
    using QueueMap for QueueMap.Queue;

    QueueMap.Queue private queue;

    struct TestStruct {
        uint256 id;
        address addr;
        string name;
        uint256[] numbers;
    }

    event QueueOperation(string operation, bytes data);

    function setUp() public {
        queue = QueueMap.createQueue("TEST_QUEUE");
    }

    // Helper functions
    function _createTestStruct(uint256 id, string memory name, uint256[] memory numbers)
        internal
        view
        returns (TestStruct memory)
    {
        return TestStruct({id: id, addr: address(this), name: name, numbers: numbers});
    }

    function _encodeStruct(TestStruct memory data) internal pure returns (bytes memory) {
        return abi.encode(data);
    }

    function _decodeStruct(bytes memory encoded) internal pure returns (TestStruct memory) {
        return abi.decode(encoded, (TestStruct));
    }

    // Basic Operations Tests
    function testInitialization() public view {
        assertTrue(queue.len() == 0);
        assertTrue(queue.isEmpty());
    }

    function testPushAndPop() public {
        uint256[] memory numbers = new uint256[](2);
        numbers[0] = 1;
        numbers[1] = 2;

        TestStruct memory data = _createTestStruct(1, "test", numbers);
        bytes memory encoded = _encodeStruct(data);

        queue.pushBack(encoded);
        assertEq(queue.len(), 1);
        assertFalse(queue.isEmpty());

        bytes memory retrieved = queue.popFront();
        TestStruct memory decoded = _decodeStruct(retrieved);

        assertEq(decoded.id, 1);
        assertEq(decoded.addr, address(this));
        assertEq(decoded.name, "test");
        assertEq(decoded.numbers[0], 1);
        assertEq(decoded.numbers[1], 2);

        assertTrue(queue.isEmpty());
    }

    function testMultiplePushPop() public {
        uint256 numItems = 5;

        // Push multiple items
        for (uint256 i = 0; i < numItems; i++) {
            uint256[] memory numbers = new uint256[](1);
            numbers[0] = i;
            TestStruct memory data = _createTestStruct(i, string(abi.encodePacked("test", i)), numbers);
            queue.pushBack(_encodeStruct(data));
        }

        assertEq(queue.len(), numItems);

        // Pop and verify all items
        for (uint256 i = 0; i < numItems; i++) {
            TestStruct memory decoded = _decodeStruct(queue.popFront());
            assertEq(decoded.id, i);
        }

        assertTrue(queue.isEmpty());
    }

    // Insert Operations Tests
    function testInsertAt() public {
        // Push initial items
        for (uint256 i = 0; i < 3; i++) {
            uint256[] memory numbers = new uint256[](1);
            numbers[0] = i;
            queue.pushBack(_encodeStruct(_createTestStruct(i, string(abi.encodePacked("test", i)), numbers)));
        }

        // Insert in middle
        uint256[] memory insertNumbers = new uint256[](1);
        insertNumbers[0] = 99;
        queue.insertAt(1, _encodeStruct(_createTestStruct(99, "inserted", insertNumbers)));

        // Verify order
        TestStruct memory first = _decodeStruct(queue.popFront());
        TestStruct memory second = _decodeStruct(queue.popFront());
        TestStruct memory third = _decodeStruct(queue.popFront());
        TestStruct memory fourth = _decodeStruct(queue.popFront());

        assertEq(first.id, 0);
        assertEq(second.id, 99);
        assertEq(third.id, 1);
        assertEq(fourth.id, 2);
    }

    function testInsertAtStart() public {
        // Push initial items
        for (uint256 i = 0; i < 3; i++) {
            uint256[] memory numbers = new uint256[](1);
            numbers[0] = i;
            queue.pushBack(_encodeStruct(_createTestStruct(i, string(abi.encodePacked("test", i)), numbers)));
        }

        // Insert at start
        uint256[] memory insertNumbers = new uint256[](1);
        insertNumbers[0] = 99;
        queue.insertAt(0, _encodeStruct(_createTestStruct(99, "inserted", insertNumbers)));

        TestStruct memory first = _decodeStruct(queue.popFront());
        assertEq(first.id, 99);
    }

    function testInsertAtEnd() public {
        // Push initial items
        for (uint256 i = 0; i < 3; i++) {
            uint256[] memory numbers = new uint256[](1);
            numbers[0] = i;
            queue.pushBack(_encodeStruct(_createTestStruct(i, string(abi.encodePacked("test", i)), numbers)));
        }

        // Insert at end
        uint256[] memory insertNumbers = new uint256[](1);
        insertNumbers[0] = 99;
        queue.insertAt(3, _encodeStruct(_createTestStruct(99, "inserted", insertNumbers)));

        // Pop all and verify last
        for (uint256 i = 0; i < 3; i++) {
            queue.popFront();
        }
        TestStruct memory last = _decodeStruct(queue.popFront());
        assertEq(last.id, 99);
    }

    // Remove Operations Tests
    function testRemoveAt() public {
        // Push initial items
        for (uint256 i = 0; i < 4; i++) {
            uint256[] memory numbers = new uint256[](1);
            numbers[0] = i;
            queue.pushBack(_encodeStruct(_createTestStruct(i, string(abi.encodePacked("test", i)), numbers)));
        }

        // Remove from middle
        TestStruct memory removed = _decodeStruct(queue.removeAt(1));
        assertEq(removed.id, 1);
        assertEq(queue.len(), 3);

        // Verify remaining order
        TestStruct memory first = _decodeStruct(queue.popFront());
        TestStruct memory second = _decodeStruct(queue.popFront());
        TestStruct memory third = _decodeStruct(queue.popFront());

        assertEq(first.id, 0);
        assertEq(second.id, 2);
        assertEq(third.id, 3);
    }

    // Error Cases Tests
    function test_PopEmptyRevert() public {
        vm.expectRevert();
        queue.popFront();
    }

    function test_InsertOutOfBoundsRevert() public {
        uint256[] memory numbers = new uint256[](1);
        vm.expectRevert();
        queue.insertAt(1, _encodeStruct(_createTestStruct(1, "test", numbers)));
    }

    function test_FailRemoveOutOfBoundsRevert() public {
        vm.expectRevert();
        queue.removeAt(0);
    }

    // Large Data Tests
    function testLargeStruct() public {
        // Create large array
        uint256[] memory numbers = new uint256[](100);
        for (uint256 i = 0; i < 100; i++) {
            numbers[i] = i;
        }

        // Create and push large struct
        TestStruct memory largeData = _createTestStruct(1, "large_test", numbers);
        queue.pushBack(_encodeStruct(largeData));

        // Retrieve and verify
        TestStruct memory retrieved = _decodeStruct(queue.popFront());
        assertEq(retrieved.numbers.length, 100);
        for (uint256 i = 0; i < 100; i++) {
            assertEq(retrieved.numbers[i], i);
        }
    }

    // Multiple Queues Test
    QueueMap.Queue private queue1;
    QueueMap.Queue private queue2;

    function testMultipleQueues() public {
        // Initialize the queues
        queue1 = QueueMap.createQueue("QUEUE_1");
        queue2 = QueueMap.createQueue("QUEUE_2");

        uint256[] memory numbers = new uint256[](1);
        numbers[0] = 1;

        // Push to both queues
        queue1.pushBack(_encodeStruct(_createTestStruct(1, "queue1", numbers)));
        queue2.pushBack(_encodeStruct(_createTestStruct(2, "queue2", numbers)));

        // Verify independence
        TestStruct memory fromQueue1 = _decodeStruct(queue1.popFront());
        TestStruct memory fromQueue2 = _decodeStruct(queue2.popFront());

        assertEq(fromQueue1.id, 1);
        assertEq(fromQueue2.id, 2);
    }

    // Stress Test
    // Main stress test function with reduced complexity
    function testStressTest() public {
        uint256 numOperations = 10000;

        for (uint256 i = 0; i < numOperations; i++) {
            uint256 operationType = i % 3;
            _handleQueueOperation(operationType, i);
        }

        _cleanupQueue();
        assertTrue(queue.isEmpty());
    }

    // Helper function to handle queue operations
    function _handleQueueOperation(uint256 operationType, uint256 i) private {
        uint256[] memory numbers = new uint256[](1);
        numbers[0] = i;

        if (operationType == 0) {
            // Push operation
            queue.pushBack(_encodeStruct(_createTestStruct(i, "stress_test", numbers)));
        } else if (operationType == 1 && !queue.isEmpty()) {
            // Pop operation
            queue.popFront();
        } else if (!queue.isEmpty()) {
            // Remove random operation
            uint256 removeIndex = i % queue.len();
            queue.removeAt(removeIndex);
        }
    }

    // Helper function to clean up the queue
    function _cleanupQueue() private {
        while (!queue.isEmpty()) {
            queue.popFront();
        }
    }
}
