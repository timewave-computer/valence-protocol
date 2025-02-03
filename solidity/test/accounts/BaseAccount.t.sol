// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";

// Test contract to receive calls from Account
contract TestTarget {
    uint256 public value;

    // Test function that accepts native tokens
    function setValueWithPayment(uint256 _value) external payable returns (uint256) {
        require(msg.value > 0, "Payment required");
        value = _value;
        return value;
    }

    // Regular function without payment
    function setValue(uint256 _value) external returns (uint256) {
        value = _value;
        return value;
    }

    // Function to test revert
    function doRevert() external pure {
        revert("TestTarget: forced revert");
    }

    // Function to check contract's native token balance
    function getBalance() public view returns (uint256) {
        return address(this).balance;
    }

    // Allow contract to receive native tokens
    receive() external payable {}
}

contract BaseAccountTest is Test {
    BaseAccount public account;
    TestTarget public target;
    address public owner;
    address public library1;
    address public library2;
    address[] public initialLibraries;

    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    function setUp() public {
        owner = address(1);
        library1 = address(2);
        library2 = address(3);

        initialLibraries = new address[](1);
        initialLibraries[0] = library1;

        account = new BaseAccount(owner, initialLibraries);
        target = new TestTarget();
    }

    function test_Constructor() public view {
        assertEq(account.owner(), owner);
        assertTrue(account.approvedLibraries(library1));
        assertFalse(account.approvedLibraries(library2));
    }

    function test_LibraryManagement() public {
        // Test adding new library
        vm.prank(owner);
        account.approveLibrary(library2);
        assertTrue(account.approvedLibraries(library2));

        // Test removing library
        vm.prank(owner);
        account.removeLibrary(library1);
        assertFalse(account.approvedLibraries(library1));
    }

    function testRevert_NonOwnerLibraryManagement() public {
        vm.prank(library1);
        account.approveLibrary(library2);
    }

    function test_ExecuteWithoutValue() public {
        uint256 testValue = 123;

        bytes memory callData = abi.encodeWithSelector(TestTarget.setValue.selector, testValue);

        vm.prank(library1);
        bytes memory result = account.execute(
            address(target),
            0, // no value sent
            callData
        );

        uint256 decodedResult = abi.decode(result, (uint256));
        assertEq(decodedResult, testValue);
        assertEq(target.value(), testValue);
    }

    function test_ExecuteWithoutValueFromOwner() public {
        uint256 testValue = 123;

        bytes memory callData = abi.encodeWithSelector(TestTarget.setValue.selector, testValue);

        vm.prank(owner);
        bytes memory result = account.execute(
            address(target),
            0, // no value sent
            callData
        );

        uint256 decodedResult = abi.decode(result, (uint256));
        assertEq(decodedResult, testValue);
        assertEq(target.value(), testValue);
    }

    function test_ExecuteWithValue() public {
        uint256 testValue = 456;
        uint256 paymentAmount = 1 ether;

        // Fund the account
        vm.deal(address(account), paymentAmount);

        bytes memory callData = abi.encodeWithSelector(TestTarget.setValueWithPayment.selector, testValue);

        vm.prank(library1);
        bytes memory result = account.execute(address(target), paymentAmount, callData);

        uint256 decodedResult = abi.decode(result, (uint256));
        assertEq(decodedResult, testValue);
        assertEq(target.value(), testValue);
        assertEq(target.getBalance(), paymentAmount);
    }

    function test_ExecuteSimpleValueTransfer() public {
        uint256 transferAmount = 1 ether;

        // Fund the account
        vm.deal(address(account), transferAmount);

        // Execute transfer with empty calldata
        vm.prank(library1);
        account.execute(address(target), transferAmount, "");

        assertEq(target.getBalance(), transferAmount);
    }

    function testRevert_ExecuteFromNonApprovedLibrary() public {
        bytes memory callData = abi.encodeWithSelector(TestTarget.setValue.selector, 123);

        vm.prank(library2);
        account.execute(address(target), 0, callData);
    }

    function testRevert_ExecuteWithInsufficientBalance() public {
        // Try to send 1 ETH when account has 0 balance
        vm.prank(library1);
        account.execute(address(target), 1 ether, "");
    }

    function test_ExecuteRevert() public {
        bytes memory callData = abi.encodeWithSelector(TestTarget.doRevert.selector);

        vm.prank(library1);
        vm.expectRevert("TestTarget: forced revert");
        account.execute(address(target), 0, callData);
    }

    function test_ReceiveNativeToken() public {
        uint256 amount = 1 ether;
        vm.deal(address(this), amount);

        // Test if contract can receive native tokens
        (bool success,) = address(account).call{value: amount}("");
        assertTrue(success);
        assertEq(address(account).balance, amount);
    }

    function test_TransferEthToUser() public {
        // Amount to transfer
        uint256 transferAmount = 1 ether;

        // Recipient address
        address recipient = address(0x1234);

        // Fund the account
        vm.deal(address(account), transferAmount);

        // Initial balance checks
        uint256 initialAccountBalance = address(account).balance;
        uint256 initialRecipientBalance = recipient.balance;

        // Execute transfer as approved library
        vm.prank(library1);
        account.execute(recipient, transferAmount, "");

        // Verify balances
        assertEq(address(account).balance, initialAccountBalance - transferAmount);
        assertEq(recipient.balance, initialRecipientBalance + transferAmount);
    }

    function test_OwnerWithdrawDepositedEth() public {
        // Amount to deposit and withdraw
        uint256 depositAmount = 2 ether;

        // Fund the account
        vm.deal(address(account), depositAmount);

        // Initial balance checks
        uint256 initialOwnerBalance = owner.balance;
        uint256 initialAccountBalance = address(account).balance;

        // Withdraw as owner
        vm.prank(owner);
        account.execute(owner, depositAmount, "");

        // Verify balances
        assertEq(address(account).balance, initialAccountBalance - depositAmount);
        assertEq(owner.balance, initialOwnerBalance + depositAmount);
    }

    function test_TransferEthToBetweenBaseAccounts() public {
        // Create a second account
        address secondOwner = address(0x5678);
        BaseAccount secondAccount = new BaseAccount(secondOwner, new address[](0));

        // Amount to transfer
        uint256 transferAmount = 1 ether;

        // Fund the first account
        vm.deal(address(account), transferAmount);

        // Initial balance checks
        uint256 initialFirstAccountBalance = address(account).balance;
        uint256 initialSecondAccountBalance = address(secondAccount).balance;

        // Execute transfer as approved library
        vm.prank(owner);
        account.execute(address(secondAccount), transferAmount, "");

        // Verify balances
        assertEq(address(account).balance, initialFirstAccountBalance - transferAmount);
        assertEq(address(secondAccount).balance, initialSecondAccountBalance + transferAmount);
    }
}
