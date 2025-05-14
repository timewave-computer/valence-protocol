// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {Authorization} from "../../src/authorization/Authorization.sol";
import {Forwarder} from "../../src/libraries/Forwarder.sol";
import {ProcessorBase} from "../../src/processor/ProcessorBase.sol";
import {LiteProcessor} from "../../src/processor/LiteProcessor.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {IProcessorMessageTypes} from "../../src/processor/interfaces/IProcessorMessageTypes.sol";
import {IProcessor} from "../../src/processor/interfaces/IProcessor.sol";

/**
 * @title AuthorizationStandardTest
 * @notice Test suite for the Authorization contract, verifying access control,
 *         processor message handling, and configuration management
 */
contract AuthorizationStandardTest is Test {
    Authorization auth;
    LiteProcessor processor;
    Forwarder forwarder;
    BaseAccount inputAccount;
    BaseAccount outputAccount;
    BaseAccount newOutputAccount;

    address owner = address(0x1);
    address admin = address(0x2);
    address user = address(0x3);
    address[] users = new address[](1);
    address[] contracts = new address[](1);
    bytes[] calls = new bytes[](1);
    address unauthorized = address(0x4);
    address mockERC20 = address(0x5);

    uint256 constant MAX_AMOUNT = 1 ether;
    uint64 constant MIN_INTERVAL = 3600;

    bytes updateConfigCall;

    function setUp() public {
        vm.startPrank(owner);

        // Deploy main contracts
        processor = new LiteProcessor(bytes32(0), address(0), 0, new address[](0));
        auth = new Authorization(owner, address(processor), address(0), true);

        // Configure processor authorization
        processor.addAuthorizedAddress(address(auth));

        // Set up accounts
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        newOutputAccount = new BaseAccount(owner, new address[](0));

        // Create and deploy forwarder with configuration
        bytes memory forwarderConfig = createForwarderConfig(address(outputAccount));
        forwarder = new Forwarder(address(processor), address(processor), forwarderConfig);

        // Set library approval
        inputAccount.approveLibrary(address(forwarder));

        // Create a new configuration for the forwarder that will be used to update
        bytes memory newForwarderConfig = createForwarderConfig(address(newOutputAccount));

        // Cache common test data
        updateConfigCall = abi.encodeWithSelector(Forwarder.updateConfig.selector, newForwarderConfig);

        // Create arrays for the batch function
        users[0] = user;
        contracts[0] = address(forwarder);
        calls[0] = updateConfigCall;

        vm.stopPrank();
    }

    // ======================= ACCESS CONTROL TESTS =======================

    function test_RevertWhen_SendProcessorMessageUnauthorized() public {
        vm.startPrank(unauthorized);

        // Create processor message with unauthorized function call
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);

        // Should fail because unauthorized user
        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(encodedMessage);

        vm.stopPrank();
    }

    function testAdminSendNonSendMsgsMessage() public {
        // Add admin address
        vm.prank(owner);
        auth.addAdminAddress(admin);

        vm.startPrank(admin);

        // Create Pause message (admin-only)
        IProcessorMessageTypes.ProcessorMessage memory processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Pause,
            message: bytes("")
        });
        bytes memory encodedMessage = abi.encode(processorMessage);

        // Execute and verify
        auth.sendProcessorMessage(encodedMessage);
        assertTrue(processor.paused(), "Processor should be paused");

        // Create Resume message (admin-only)
        processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Resume,
            message: bytes("")
        });
        encodedMessage = abi.encode(processorMessage);

        // Execute and verify
        auth.sendProcessorMessage(encodedMessage);
        assertFalse(processor.paused(), "Processor should be resumed");

        vm.stopPrank();
    }

    function test_RevertWhen_NonAdminSendNonSendMsgsMessage() public {
        vm.startPrank(user);

        // Create Pause message (admin-only)
        IProcessorMessageTypes.ProcessorMessage memory processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Pause,
            message: bytes("")
        });
        bytes memory encodedMessage = abi.encode(processorMessage);

        // Should fail because user is not admin
        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(encodedMessage);

        vm.stopPrank();
    }

    // ======================= CONFIGURATION TESTS =======================

    function testUpdateProcessor() public {
        vm.startPrank(owner);

        // Deploy a new processor
        ProcessorBase newProcessor = new LiteProcessor(bytes32(0), address(0), 0, new address[](0));

        // Update and verify
        auth.updateProcessor(address(newProcessor));
        assertEq(address(auth.processor()), address(newProcessor), "Processor should be updated");

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateProcessorWithZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert("Processor cannot be zero address");
        auth.updateProcessor(address(0));
    }

    /**
     * @notice Test updating the verification gateway address
     */
    function testUpdateVerificationGateway() public {
        vm.startPrank(owner);

        address newVerificationGateway = address(0x9);
        auth.updateVerificationGateway(newVerificationGateway);
        assertEq(address(auth.verificationGateway()), newVerificationGateway, "Verification Gateway should be updated");

        vm.stopPrank();
    }

    function test_RevertWhen_HandleCallbackUnauthorized() public {
        vm.startPrank(unauthorized);

        // Create a callback
        IProcessor.Callback memory callback = IProcessor.Callback({
            executionId: 42,
            executionResult: IProcessor.ExecutionResult.Success,
            executedCount: 1,
            data: abi.encode("Test callback data")
        });
        bytes memory callbackData = abi.encode(callback);

        // Should fail because only processor can call
        vm.expectRevert("Only processor can call this function");
        auth.handleCallback(callbackData);

        vm.stopPrank();
    }

    // ======================= ADMIN MANAGEMENT TESTS =======================

    /**
     * @notice Test adding an admin address
     */
    function testAddAdminAddress() public {
        vm.prank(owner);
        auth.addAdminAddress(admin);
        assertTrue(auth.adminAddresses(admin), "Admin address should be authorized");
    }

    /**
     * @notice Test removing an admin address
     */
    function testRemoveAdminAddress() public {
        vm.startPrank(owner);

        // Add and verify admin
        auth.addAdminAddress(admin);
        assertTrue(auth.adminAddresses(admin), "Admin address should be authorized");

        // Remove and verify admin
        auth.removeAdminAddress(admin);
        assertFalse(auth.adminAddresses(admin), "Admin address should no longer be authorized");

        vm.stopPrank();
    }

    /**
     * @notice Test that only owner can add admin addresses
     */
    function test_RevertWhen_AddAdminAddressUnauthorized() public {
        vm.prank(unauthorized);
        vm.expectRevert();
        auth.addAdminAddress(admin);
    }

    // ======================= AUTHORIZATION MANAGEMENT TESTS =======================

    /**
     * @notice Test adding a standard user authorization
     */
    function testAddStandardAuthorization() public {
        vm.prank(owner);
        auth.addStandardAuthorizations(users, contracts, calls);

        assertTrue(
            auth.authorizations(user, address(forwarder), keccak256(updateConfigCall)),
            "Authorization should be granted"
        );
    }

    /**
     * @notice Test adding a permissionless authorization (zero address)
     */
    function testAddPermissionlessAuthorization() public {
        users[0] = address(0);
        vm.prank(owner);
        auth.addStandardAuthorizations(users, contracts, calls);

        assertTrue(
            auth.authorizations(address(0), address(forwarder), keccak256(updateConfigCall)),
            "Permissionless authorization should be granted"
        );
    }

    function test_RevertWhen_AddingInvalidAuthorization() public {
        // Create arrays for the batch function
        address[] memory invalidUsers = new address[](2);
        invalidUsers[0] = user;
        invalidUsers[1] = address(0);

        vm.prank(owner);
        vm.expectRevert("Array lengths must match");
        auth.addStandardAuthorizations(invalidUsers, contracts, calls);
    }

    /**
     * @notice Test removing a standard authorization
     */
    function testRemoveStandardAuthorization() public {
        vm.startPrank(owner);

        // Add authorization
        auth.addStandardAuthorizations(users, contracts, calls);

        // Remove authorization and verify
        auth.removeStandardAuthorizations(users, contracts, calls);
        assertFalse(
            auth.authorizations(user, address(forwarder), keccak256(updateConfigCall)),
            "Authorization should be removed"
        );

        vm.stopPrank();
    }

    // ======================= PROCESSOR MESSAGE TESTS =======================

    function testSendProcessorMessageAuthorized() public {
        // Authorize user
        vm.prank(owner);
        auth.addStandardAuthorizations(users, contracts, calls);

        vm.startPrank(user);

        // Create and send processor message
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);
        auth.sendProcessorMessage(encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that Forwarder config was updated
        (, BaseAccount updatedOutputAccount,,) = forwarder.config();
        assertEq(address(updatedOutputAccount), address(newOutputAccount), "Output account should be updated");

        // Verify that we got a callback
        (IProcessor.ExecutionResult result, uint64 executionCount, bytes memory data) = auth.callbacks(0);
        assert(result == IProcessor.ExecutionResult.Success);
        assertEq(executionCount, 1, "Execution count should be 1");
        assertEq(data, bytes(""), "Callback data should be empty");

        vm.stopPrank();
    }

    function testSendProcessorMessagePermissionless() public {
        // Add permissionless authorization
        vm.prank(owner);
        users[0] = address(0);
        auth.addStandardAuthorizations(users, contracts, calls);

        vm.startPrank(unauthorized);

        // Create and send processor message
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);
        auth.sendProcessorMessage(encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        vm.stopPrank();
    }

    // ======================= HELPER FUNCTIONS =======================

    /**
     * @notice Create a forwarder configuration for testing
     * @return Encoded forwarder configuration bytes
     */
    function createForwarderConfig(address _outputAccount) public view returns (bytes memory) {
        // Create ERC20 forwarding configuration
        Forwarder.ForwardingConfig[] memory configs = new Forwarder.ForwardingConfig[](1);
        configs[0] = Forwarder.ForwardingConfig({tokenAddress: mockERC20, maxAmount: MAX_AMOUNT});

        // Create complete forwarder config
        Forwarder.ForwarderConfig memory config = Forwarder.ForwarderConfig({
            inputAccount: inputAccount,
            outputAccount: BaseAccount(payable(_outputAccount)),
            forwardingConfigs: configs,
            intervalType: Forwarder.IntervalType.TIME,
            minInterval: MIN_INTERVAL
        });

        return abi.encode(config);
    }

    /**
     * @notice Creates a SendMsgs processor message with the given function call
     * @param functionCall The encoded function call to include in the message
     * @return Encoded processor message bytes
     */
    function createSendMsgsMessage(bytes memory functionCall) internal view returns (bytes memory) {
        // Create atomic subroutine retry logic
        IProcessorMessageTypes.RetryTimes memory times =
            IProcessorMessageTypes.RetryTimes({retryType: IProcessorMessageTypes.RetryTimesType.Amount, amount: 3});

        IProcessorMessageTypes.Duration memory duration =
            IProcessorMessageTypes.Duration({durationType: IProcessorMessageTypes.DurationType.Time, value: 0});

        IProcessorMessageTypes.RetryLogic memory retryLogic =
            IProcessorMessageTypes.RetryLogic({times: times, interval: duration});

        // Create atomic subroutine with forwarder function
        IProcessorMessageTypes.AtomicFunction memory atomicFunction =
            IProcessorMessageTypes.AtomicFunction({contractAddress: address(forwarder)});

        IProcessorMessageTypes.AtomicFunction[] memory atomicFunctions = new IProcessorMessageTypes.AtomicFunction[](1);
        atomicFunctions[0] = atomicFunction;

        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            IProcessorMessageTypes.AtomicSubroutine({functions: atomicFunctions, retryLogic: retryLogic});

        // Create subroutine wrapper
        IProcessorMessageTypes.Subroutine memory subroutine = IProcessorMessageTypes.Subroutine({
            subroutineType: IProcessorMessageTypes.SubroutineType.Atomic,
            subroutine: abi.encode(atomicSubroutine)
        });

        // Create messages array with function call
        bytes[] memory messages = new bytes[](1);
        messages[0] = functionCall;

        // Create SendMsgs message
        IProcessorMessageTypes.SendMsgs memory sendMsgs = IProcessorMessageTypes.SendMsgs({
            subroutine: subroutine,
            messages: messages,
            expirationTime: 0,
            executionId: 0, // Will be set by Authorization contract
            priority: IProcessorMessageTypes.Priority.Medium
        });

        // Create and encode processor message
        IProcessorMessageTypes.ProcessorMessage memory processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.SendMsgs,
            message: abi.encode(sendMsgs)
        });

        return abi.encode(processorMessage);
    }
}
