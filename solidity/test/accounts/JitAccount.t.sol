// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import "../../src/accounts/JitAccount.sol";

contract MockTarget {
    event Called(address sender, uint256 value, bytes data);

    function simpleCall() external payable {
        emit Called(msg.sender, msg.value, msg.data);
    }

    function revertingCall() external pure {
        revert("Mock revert");
    }

    // Allow contract to receive ETH
    receive() external payable {}
}

contract JitAccountTest is Test {
    JitAccount public account;
    MockTarget public target;

    address public controller = address(0x1);
    address public lib = address(0x2);
    address public unauthorizedUser = address(0x3);

    function setUp() public {
        account = new JitAccount(controller);
        target = new MockTarget();

        // Fund the account for testing
        vm.deal(address(account), 1 ether);
    }

    function test_constructor() public {
        assertEq(account.controller(), controller);
        assertEq(account.getController(), controller);
    }

    function test_approveLibrary_onlyController() public {
        // Controller can approve library
        vm.prank(controller);
        account.approveLibrary(lib);
        assertTrue(account.isLibraryApproved(lib));

        // Non-controller cannot approve library
        vm.prank(unauthorizedUser);
        vm.expectRevert(JitAccount.Unauthorized.selector);
        account.approveLibrary(lib);
    }

    function test_removeLibrary_onlyController() public {
        // First approve a library
        vm.prank(controller);
        account.approveLibrary(lib);
        assertTrue(account.isLibraryApproved(lib));

        // Controller can remove library
        vm.prank(controller);
        account.removeLibrary(lib);
        assertFalse(account.isLibraryApproved(lib));

        // Non-controller cannot remove library
        vm.prank(controller);
        account.approveLibrary(lib);

        vm.prank(unauthorizedUser);
        vm.expectRevert(JitAccount.Unauthorized.selector);
        account.removeLibrary(lib);
    }

    function test_execute_controllerAccess() public {
        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        targets[0] = address(target);
        data[0] = abi.encodeWithSignature("simpleCall()");
        values[0] = 0;

        // Controller can execute
        vm.prank(controller);
        account.execute(targets, data, values);
    }

    function test_execute_approvedLibraryAccess() public {
        // Approve library first
        vm.prank(controller);
        account.approveLibrary(lib);

        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        targets[0] = address(target);
        data[0] = abi.encodeWithSignature("simpleCall()");
        values[0] = 0;

        // Approved library can execute
        vm.prank(lib);
        account.execute(targets, data, values);
    }

    function test_execute_unauthorizedAccess() public {
        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        targets[0] = address(target);
        data[0] = abi.encodeWithSignature("simpleCall()");
        values[0] = 0;

        // Unauthorized user cannot execute
        vm.prank(unauthorizedUser);
        vm.expectRevert(JitAccount.Unauthorized.selector);
        account.execute(targets, data, values);
    }

    function test_execute_arrayLengthMismatch() public {
        address[] memory targets = new address[](2);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        vm.prank(controller);
        vm.expectRevert("Array length mismatch");
        account.execute(targets, data, values);
    }

    function test_execute_failedCall() public {
        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        targets[0] = address(target);
        data[0] = abi.encodeWithSignature("revertingCall()");
        values[0] = 0;

        vm.prank(controller);
        vm.expectRevert();
        account.execute(targets, data, values);
    }

    function test_events() public {
        // Test LibraryApproved event
        vm.prank(controller);
        vm.expectEmit(true, false, false, false);
        emit JitAccount.LibraryApproved(lib);
        account.approveLibrary(lib);

        // Test LibraryRemoved event
        vm.prank(controller);
        vm.expectEmit(true, false, false, false);
        emit JitAccount.LibraryRemoved(lib);
        account.removeLibrary(lib);

        // Test MessagesExecuted event
        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);
        uint256[] memory values = new uint256[](1);

        targets[0] = address(target);
        data[0] = abi.encodeWithSignature("simpleCall()");
        values[0] = 0;

        vm.prank(controller);
        vm.expectEmit(true, false, false, true);
        emit JitAccount.MessagesExecuted(controller, 1);
        account.execute(targets, data, values);
    }

    function test_receiveEther() public {
        uint256 initialBalance = address(account).balance;

        // Send ETH to account
        payable(address(account)).transfer(0.5 ether);

        assertEq(address(account).balance, initialBalance + 0.5 ether);
    }
}
