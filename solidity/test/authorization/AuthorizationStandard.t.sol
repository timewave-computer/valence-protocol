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
    address[][] users = new address[][](1);
    string[] labels = new string[](1);
    string updateConfigLabel = "updateConfig";
    string forwardLabel = "forward";
    Authorization.AuthorizationData[][] authorizationData = new Authorization.AuthorizationData[][](1);
    bytes[] calls = new bytes[](1);
    address unauthorized = address(0x4);
    address mockERC20 = address(0x5);

    uint256 constant MAX_AMOUNT = 1 ether;
    uint64 constant MIN_INTERVAL = 3600;

    bytes updateConfigCall;
    bytes forwardCall;

    function setUp() public {
        // Set initial block timestamp and height
        vm.warp(5000);
        vm.roll(100);

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
        forwardCall = abi.encodeWithSelector(Forwarder.forward.selector);

        // Create arrays for the batch function
        users[0] = new address[](1);
        users[0][0] = user;
        labels[0] = updateConfigLabel;
        authorizationData[0] = new Authorization.AuthorizationData[](1);
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: false,
            functionSelector: bytes4(0),
            callHash: keccak256(updateConfigCall)
        });

        vm.stopPrank();
    }

    // ======================= ACCESS CONTROL TESTS =======================

    function test_RevertWhen_SendProcessorMessageUnauthorized() public {
        vm.startPrank(unauthorized);

        // Create processor message with unauthorized function call
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);

        // Should fail because unauthorized user
        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage("anyLabel", encodedMessage);

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
        auth.sendProcessorMessage("anyLabel", encodedMessage);
        assertTrue(processor.paused(), "Processor should be paused");

        // Create Resume message (admin-only)
        processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Resume,
            message: bytes("")
        });
        encodedMessage = abi.encode(processorMessage);

        // Execute and verify
        auth.sendProcessorMessage("anyLabel", encodedMessage);
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
        auth.sendProcessorMessage("anyLabel", encodedMessage);

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
     * @notice Test adding a standard user authorization using callHash
     */
    function testAddStandardAuthorizationWithCallHash() public {
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        address labelAddress = auth.authorizations(labels[0], 0);
        assertEq(labelAddress, users[0][0], "User should be authorized");
    }

    /**
     * @notice Test adding a standard user authorization using function selector
     */
    function testAddStandardAuthorizationWithFunctionSelector() public {
        // Create authorization data using function selector
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0) // Not used when useFunctionSelector is true
        });

        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        address labelAddress = auth.authorizations(labels[0], 0);
        assertEq(labelAddress, users[0][0], "User should be authorized");
    }

    /**
     * @notice Test adding a permissionless authorization (zero address)
     */
    function testAddPermissionlessAuthorization() public {
        users[0][0] = address(0);
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        address labelAddress = auth.authorizations(labels[0], 0);
        assertEq(labelAddress, address(0), "Permissionless access should be authorized");
    }

    function test_RevertWhen_AddingInvalidAuthorization() public {
        // Create arrays for the batch function
        address[][] memory invalidUsers = new address[][](2);
        invalidUsers[0] = new address[](1);
        invalidUsers[0][0] = user;
        invalidUsers[1] = new address[](0);

        vm.prank(owner);
        vm.expectRevert("Array lengths must match");
        auth.addStandardAuthorizations(labels, invalidUsers, authorizationData);
    }

    /**
     * @notice Test removing a standard authorization
     */
    function testRemoveStandardAuthorization() public {
        vm.startPrank(owner);

        // Add authorization
        auth.addStandardAuthorizations(labels, users, authorizationData);

        // Remove authorization and verify
        auth.removeStandardAuthorizations(labels);

        vm.expectRevert();
        auth.authorizations(labels[0], 0); // This should revert if the array is empty

        vm.stopPrank();
    }

    // ======================= PROCESSOR MESSAGE TESTS =======================

    function testSendProcessorMessageAuthorizedWithCallHash() public {
        // Authorize user with callHash
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Create and send processor message
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that Forwarder config was updated
        (, BaseAccount updatedOutputAccount,,) = forwarder.config();
        assertEq(address(updatedOutputAccount), address(newOutputAccount), "Output account should be updated");

        vm.stopPrank();
    }

    function testSendProcessorMessageAuthorizedWithFunctionSelector() public {
        // Create authorization data using function selector
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });

        // Authorize user with function selector
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Create and send processor message
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that Forwarder config was updated
        (, BaseAccount updatedOutputAccount,,) = forwarder.config();
        assertEq(address(updatedOutputAccount), address(newOutputAccount), "Output account should be updated");

        vm.stopPrank();
    }

    function testSendProcessorMessagePermissionless() public {
        // Add permissionless authorization
        vm.prank(owner);
        users[0][0] = address(0);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(unauthorized);

        // Create and send processor message
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        vm.stopPrank();
    }

    // ======================= NEGATIVE TESTS =======================

    /**
     * @notice Test that wrong function selector fails when using function selector validation
     */
    function test_RevertWhen_WrongFunctionSelectorUsed() public {
        // Create authorization data using function selector for updateConfig
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector, // Authorized for updateConfig only
            callHash: bytes32(0)
        });

        // Authorize user with function selector
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Try to call a different function (forward instead of updateConfig)
        bytes memory wrongFunctionMessage = createSendMsgsMessage(forwardCall);

        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(updateConfigLabel, wrongFunctionMessage);

        vm.stopPrank();
    }

    /**
     * @notice Test that wrong callHash fails when using callHash validation
     */
    function test_RevertWhen_WrongCallHashUsed() public {
        // Create authorization data using callHash for updateConfig
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: false,
            functionSelector: bytes4(0),
            callHash: keccak256(updateConfigCall) // Authorized for specific updateConfig call only
        });

        // Authorize user with callHash
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Try to call with different parameters (different callHash)
        bytes memory differentParamsCall = abi.encodeWithSelector(
            Forwarder.updateConfig.selector,
            createForwarderConfig(address(0x99)) // Different parameters
        );
        bytes memory wrongCallHashMessage = createSendMsgsMessage(differentParamsCall);

        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(updateConfigLabel, wrongCallHashMessage);

        vm.stopPrank();
    }

    /**
     * @notice Test that wrong contract address fails
     */
    function test_RevertWhen_WrongContractAddress() public {
        // Create authorization data for wrong contract address
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(0x999), // Wrong contract address
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });

        // Authorize user with wrong contract address
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Try to call the correct contract (should fail because authorization is for different contract)
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);

        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        vm.stopPrank();
    }

    /**
     * @notice Test that mismatched authorization data length fails
     */
    function test_RevertWhen_MismatchedAuthorizationDataLength() public {
        // Create authorization data with 2 elements
        Authorization.AuthorizationData[][] memory mismatchedAuthData = new Authorization.AuthorizationData[][](1);
        mismatchedAuthData[0] = new Authorization.AuthorizationData[](2);
        mismatchedAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });
        mismatchedAuthData[0][1] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.forward.selector,
            callHash: bytes32(0)
        });

        // Authorize user with 2 authorization data elements
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, mismatchedAuthData);

        vm.startPrank(user);

        // Try to call with only 1 function (should fail because authorization expects 2)
        bytes memory encodedMessage = createSendMsgsMessage(updateConfigCall);

        vm.expectRevert("Unauthorized access");
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        vm.stopPrank();
    }

    /**
     * @notice Test that function selector works correctly when the same function has different parameters
     */
    function testFunctionSelectorIgnoresParameters() public {
        // Create authorization data using function selector
        authorizationData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });

        // Authorize user with function selector
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, authorizationData);

        vm.startPrank(user);

        // Create different updateConfig call with different parameters
        bytes memory differentParamsCall = abi.encodeWithSelector(
            Forwarder.updateConfig.selector,
            createForwarderConfig(address(0x99)) // Different parameters but same function selector
        );
        bytes memory encodedMessage = createSendMsgsMessage(differentParamsCall);

        // This should succeed because function selector matches, parameters are ignored
        auth.sendProcessorMessage(updateConfigLabel, encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that Forwarder config was updated
        (, BaseAccount updatedOutputAccount,,) = forwarder.config();
        assertEq(address(updatedOutputAccount), address(0x99), "Output account should be updated");

        vm.stopPrank();
    }

    /**
     * @notice Test multiple functions in atomic subroutine with mixed validation types
     */
    function testMultipleFunctionsWithMixedValidation() public {
        // Create authorization data with mixed validation types
        Authorization.AuthorizationData[][] memory mixedAuthData = new Authorization.AuthorizationData[][](1);
        mixedAuthData[0] = new Authorization.AuthorizationData[](2);
        mixedAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });
        mixedAuthData[0][1] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: false,
            functionSelector: bytes4(0),
            callHash: keccak256(forwardCall)
        });

        // Authorize user with mixed validation
        vm.prank(owner);
        auth.addStandardAuthorizations(labels, users, mixedAuthData);

        vm.startPrank(user);

        // Create message with both functions
        bytes memory dualFunctionMessage = createDualFunctionMessageAtomicSubroutine(updateConfigCall, forwardCall);

        // This should succeed because both validations match
        auth.sendProcessorMessage(updateConfigLabel, dualFunctionMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        vm.stopPrank();
    }

    // ======================= EXECUTION FAILURE TESTS =======================

    /**
     * @notice Test that failed execution returns proper callback with failure result
     */
    function testSendProcessorMessageWithFailedExecutionNoFunds() public {
        // Create authorization for forward function on forwarder
        string[] memory forwardLabels = new string[](1);
        forwardLabels[0] = "forward";

        address[][] memory forwardUsers = new address[][](1);
        forwardUsers[0] = new address[](1);
        forwardUsers[0][0] = user;

        Authorization.AuthorizationData[][] memory forwardAuthData = new Authorization.AuthorizationData[][](1);
        forwardAuthData[0] = new Authorization.AuthorizationData[](1);
        forwardAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.forward.selector,
            callHash: bytes32(0)
        });

        // Authorize user for forward function
        vm.prank(owner);
        auth.addStandardAuthorizations(forwardLabels, forwardUsers, forwardAuthData);

        vm.startPrank(user);

        // Call will fail because not enough balance
        bytes memory encodedMessage = createSendMsgsMessage(forwardCall);

        // Execute the message (should not revert but execution should fail)
        auth.sendProcessorMessage("forward", encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that we got a callback with failure result
        (IProcessor.ExecutionResult result, uint64 executionCount,) = auth.callbacks(0);

        // The execution should have failed
        assertEq(uint256(result), uint256(IProcessor.ExecutionResult.Rejected), "Execution should have failed");
        assertEq(executionCount, 0, "Execution count should be 0 for failed execution");

        vm.stopPrank();
    }

    /**
     * @notice Test that execution with revert and returns proper callback
     */
    function testSendProcessorMessageNonExistentFunction() public {
        // Create authorization for a function that will revert
        string[] memory revertLabels = new string[](1);
        revertLabels[0] = "revertFunction";

        address[][] memory revertUsers = new address[][](1);
        revertUsers[0] = new address[](1);
        revertUsers[0][0] = user;

        Authorization.AuthorizationData[][] memory revertAuthData = new Authorization.AuthorizationData[][](1);
        revertAuthData[0] = new Authorization.AuthorizationData[](1);
        revertAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: bytes4(keccak256("nonExistentFunction()")), // Function that doesn't exist
            callHash: bytes32(0)
        });

        // Authorize user for non-existent function
        vm.prank(owner);
        auth.addStandardAuthorizations(revertLabels, revertUsers, revertAuthData);

        vm.startPrank(user);

        // Create a call to non-existent function
        bytes memory revertingCall = abi.encodeWithSelector(bytes4(keccak256("nonExistentFunction()")));

        bytes memory encodedMessage = createSendMsgsMessage(revertingCall);

        // Execute the message (should not revert but execution should fail)
        auth.sendProcessorMessage("revertFunction", encodedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify that we got a callback with failure result
        (IProcessor.ExecutionResult result, uint64 executionCount,) = auth.callbacks(0);

        // The execution should have failed
        assertEq(uint256(result), uint256(IProcessor.ExecutionResult.Rejected), "Execution should have failed");
        assertEq(executionCount, 0, "Execution count should be 0 for failed execution");

        vm.stopPrank();
    }

    /**
     * @notice Test execution failure in atomic subroutine
     */
    function testExecutionFailureInAtomicSubroutine() public {
        // Create authorization for multiple functions where one will fail
        string[] memory mixedLabels = new string[](1);
        mixedLabels[0] = "mixedExecution";

        address[][] memory mixedUsers = new address[][](1);
        mixedUsers[0] = new address[](1);
        mixedUsers[0][0] = user;

        Authorization.AuthorizationData[][] memory mixedAuthData = new Authorization.AuthorizationData[][](1);
        mixedAuthData[0] = new Authorization.AuthorizationData[](2);

        // First function: updateConfig (should succeed)
        mixedAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });

        // Second function: failing forward (should fail), not enough balance
        mixedAuthData[0][1] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.forward.selector,
            callHash: bytes32(0)
        });

        // Authorize user for mixed execution
        vm.prank(owner);
        auth.addStandardAuthorizations(mixedLabels, mixedUsers, mixedAuthData);

        vm.startPrank(user);

        // Create message with both functions
        bytes memory mixedMessage = createDualFunctionMessageAtomicSubroutine(updateConfigCall, forwardCall);

        // Execute the message
        auth.sendProcessorMessage("mixedExecution", mixedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify callback result
        (IProcessor.ExecutionResult result, uint64 executionCount,) = auth.callbacks(0);

        // In atomic subroutine, if one fails, the whole execution should fail
        // But the executed count might indicate how many succeeded before failure
        assertEq(uint256(result), uint256(IProcessor.ExecutionResult.Rejected), "Execution should have failed");

        // The executionCount should be 0 because the in atomic subroutine all functions must succeed
        assertEq(executionCount, 0, "Execution count should be 0 for failed execution");

        vm.stopPrank();
    }

    /**
     * @notice Test partial execution failure in non atomic subroutine
     */
    function testPartialExecutionFailureInNonAtomicSubroutine() public {
        // Create authorization for multiple functions where one will fail
        string[] memory mixedLabels = new string[](1);
        mixedLabels[0] = "mixedExecution";

        address[][] memory mixedUsers = new address[][](1);
        mixedUsers[0] = new address[](1);
        mixedUsers[0][0] = user;

        Authorization.AuthorizationData[][] memory mixedAuthData = new Authorization.AuthorizationData[][](1);
        mixedAuthData[0] = new Authorization.AuthorizationData[](2);

        // First function: updateConfig (should succeed)
        mixedAuthData[0][0] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.updateConfig.selector,
            callHash: bytes32(0)
        });

        // Second function: failing forward (should fail), not enough balance
        mixedAuthData[0][1] = Authorization.AuthorizationData({
            contractAddress: address(forwarder),
            useFunctionSelector: true,
            functionSelector: Forwarder.forward.selector,
            callHash: bytes32(0)
        });

        // Authorize user for mixed execution
        vm.prank(owner);
        auth.addStandardAuthorizations(mixedLabels, mixedUsers, mixedAuthData);

        vm.startPrank(user);

        // Create message with both functions
        bytes memory mixedMessage = createDualFunctionMessageNonAtomicSubroutine(updateConfigCall, forwardCall);

        // Execute the message
        auth.sendProcessorMessage("mixedExecution", mixedMessage);

        // Verify executionId was incremented
        assertEq(auth.executionId(), 1, "Execution ID should be incremented");

        // Verify callback result
        (IProcessor.ExecutionResult result, uint64 executionCount,) = auth.callbacks(0);

        // In NonAtomic subroutine, we process functions until one fails
        assertEq(
            uint256(result),
            uint256(IProcessor.ExecutionResult.PartiallyExecuted),
            "Execution should have been partially executed"
        );

        // The executionCount should be 1 because 1 should have succeeded (the update config)
        assertEq(executionCount, 1, "Execution count should be 1 because the update config succeeded");

        // Verify that the output account was updated
        (, BaseAccount updatedOutputAccount,,) = forwarder.config();
        assertEq(address(updatedOutputAccount), address(newOutputAccount), "Output account should be updated");

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

    /**
     * @notice Creates a SendMsgs processor message with two function calls for an atomic subroutine
     * @param functionCall1 The first encoded function call
     * @param functionCall2 The second encoded function call
     * @return Encoded processor message bytes
     */
    function createDualFunctionMessageAtomicSubroutine(bytes memory functionCall1, bytes memory functionCall2)
        internal
        view
        returns (bytes memory)
    {
        // Create atomic subroutine retry logic
        IProcessorMessageTypes.RetryTimes memory times =
            IProcessorMessageTypes.RetryTimes({retryType: IProcessorMessageTypes.RetryTimesType.Amount, amount: 3});

        IProcessorMessageTypes.Duration memory duration =
            IProcessorMessageTypes.Duration({durationType: IProcessorMessageTypes.DurationType.Time, value: 0});

        IProcessorMessageTypes.RetryLogic memory retryLogic =
            IProcessorMessageTypes.RetryLogic({times: times, interval: duration});

        // Create atomic subroutine with two functions
        IProcessorMessageTypes.AtomicFunction[] memory atomicFunctions = new IProcessorMessageTypes.AtomicFunction[](2);
        atomicFunctions[0] = IProcessorMessageTypes.AtomicFunction({contractAddress: address(forwarder)});
        atomicFunctions[1] = IProcessorMessageTypes.AtomicFunction({contractAddress: address(forwarder)});

        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            IProcessorMessageTypes.AtomicSubroutine({functions: atomicFunctions, retryLogic: retryLogic});

        // Create subroutine wrapper
        IProcessorMessageTypes.Subroutine memory subroutine = IProcessorMessageTypes.Subroutine({
            subroutineType: IProcessorMessageTypes.SubroutineType.Atomic,
            subroutine: abi.encode(atomicSubroutine)
        });

        // Create messages array with both function calls
        bytes[] memory messages = new bytes[](2);
        messages[0] = functionCall1;
        messages[1] = functionCall2;

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

    /**
     * @notice Creates a SendMsgs processor message with two function calls for a non atomic subroutine
     * @param functionCall1 The first encoded function call
     * @param functionCall2 The second encoded function call
     * @return Encoded processor message bytes
     */
    function createDualFunctionMessageNonAtomicSubroutine(bytes memory functionCall1, bytes memory functionCall2)
        internal
        view
        returns (bytes memory)
    {
        // Create non atomic function retry logic
        IProcessorMessageTypes.RetryTimes memory times =
            IProcessorMessageTypes.RetryTimes({retryType: IProcessorMessageTypes.RetryTimesType.Amount, amount: 3});

        IProcessorMessageTypes.Duration memory duration =
            IProcessorMessageTypes.Duration({durationType: IProcessorMessageTypes.DurationType.Time, value: 0});

        IProcessorMessageTypes.RetryLogic memory retryLogic =
            IProcessorMessageTypes.RetryLogic({times: times, interval: duration});

        // Create function callback
        IProcessorMessageTypes.FunctionCallback memory functionCallback =
            IProcessorMessageTypes.FunctionCallback({contractAddress: address(0), callbackMessage: bytes("")});

        // Create atomic subroutine with two functions
        IProcessorMessageTypes.NonAtomicFunction[] memory nonAtomicFunctions =
            new IProcessorMessageTypes.NonAtomicFunction[](2);
        nonAtomicFunctions[0] = IProcessorMessageTypes.NonAtomicFunction({
            contractAddress: address(forwarder),
            retryLogic: retryLogic,
            callbackConfirmation: functionCallback
        });
        nonAtomicFunctions[1] = IProcessorMessageTypes.NonAtomicFunction({
            contractAddress: address(forwarder),
            retryLogic: retryLogic,
            callbackConfirmation: functionCallback
        });

        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
            IProcessorMessageTypes.NonAtomicSubroutine({functions: nonAtomicFunctions});

        // Create subroutine wrapper
        IProcessorMessageTypes.Subroutine memory subroutine = IProcessorMessageTypes.Subroutine({
            subroutineType: IProcessorMessageTypes.SubroutineType.NonAtomic,
            subroutine: abi.encode(nonAtomicSubroutine)
        });

        // Create messages array with both function calls
        bytes[] memory messages = new bytes[](2);
        messages[0] = functionCall1;
        messages[1] = functionCall2;

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
