// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {Authorization} from "../../src/authorization/Authorization.sol";
import {VerificationRouter} from "../../src/verification/VerificationRouter.sol";
import {SP1VerificationSwitch} from "../../src/verification/SP1VerificationSwitch.sol";
import {ProcessorBase} from "../../src/processor/ProcessorBase.sol";
import {LiteProcessor} from "../../src/processor/LiteProcessor.sol";
import {IProcessorMessageTypes} from "../../src/processor/interfaces/IProcessorMessageTypes.sol";

/**
 * @title AuthorizationZKTest
 * @notice Test suite for the ZK authorization flow in the Authorization contract
 * @dev Tests focus on registry management and unauthorized access verification
 */
contract AuthorizationZKTest is Test {
    Authorization auth;
    LiteProcessor processor;
    VerificationRouter verificationRouter;
    SP1VerificationSwitch sp1VerificationSwitch;

    address owner = address(0x1);
    address user1 = address(0x2);
    address user2 = address(0x3);
    address unauthorized = address(0x4);
    string route = "route66";
    bytes32 coprocessorRoot = bytes32(uint256(0x42069));

    // ZK registry configuration
    uint64 registryId1 = 101;
    uint64 registryId2 = 102;
    bytes vk1 = abi.encodePacked(bytes32(uint256(0x123456)));
    bytes vk2 = abi.encodePacked(bytes32(uint256(0x789abc)));
    bool validateBlockNumber1 = false;
    bool validateBlockNumber2 = true;

    function setUp() public {
        vm.startPrank(owner);

        // Deploy processor
        processor = new LiteProcessor(bytes32(0), address(0), 0, new address[](0));

        // Deploy a SP1 verification switch
        sp1VerificationSwitch = new SP1VerificationSwitch(
            address(0x5), // Mock SP1 verifier address
            bytes32(uint256(0xdeadbeef)) // Mock domain verification key
        );

        // Deploy verification router
        verificationRouter = new VerificationRouter();

        // Add a route for the verification router
        verificationRouter.addRoute(route, address(sp1VerificationSwitch));

        // Deploy authorization contract
        auth = new Authorization(owner, address(processor), true);

        // Set the verification router
        auth.setVerificationRouter(address(verificationRouter));

        // Configure processor to accept messages from auth contract
        processor.addAuthorizedAddress(address(auth));

        vm.stopPrank();
    }

    // ======================= REGISTRY MANAGEMENT TESTS =======================

    /**
     * @notice Test adding registries with verification keys and authorized users
     */
    function testAddRegistries() public {
        vm.startPrank(owner);

        // Create registry data
        uint64[] memory registries = new uint64[](2);
        registries[0] = registryId1;
        registries[1] = registryId2;

        // Create arrays of authorized users for each registry
        address[][] memory users = new address[][](2);

        // Registry 1: user1 and user2 authorized
        users[0] = new address[](2);
        users[0][0] = user1;
        users[0][1] = user2;

        // Registry 2: permissionless (address(0))
        users[1] = new address[](1);
        users[1][0] = address(0); // Permissionless access

        // Create two ZkAuthorizationData objects
        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](2);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        zkAuthData[1] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[1],
            route: route,
            vk: vk2,
            validateBlockNumberExecution: validateBlockNumber2,
            metadataHash: bytes32(0)
        });

        // Add registries
        auth.addRegistries(registries, zkAuthData);

        // Verify registry 1
        Authorization.ZkAuthorizationData memory authData1 = auth.getZkAuthorizationData(registryId1);
        assertEq(authData1.vk, vk1, "Verification key for registry 1 should be stored correctly");
        assertEq(authData1.allowedExecutionAddresses.length, 2, "Registry 1 should have two authorized users");
        assertEq(authData1.allowedExecutionAddresses[0], user1, "Registry 1 should authorize user1");
        assertEq(authData1.allowedExecutionAddresses[1], user2, "Registry 1 should authorize user2");
        assertEq(authData1.route, route, "Registry 1 should have the correct route");
        assertEq(
            auth.zkAuthorizationLastExecutionBlock(registryId1), 0, "Last execution block should be zero for registry 1"
        );

        // Verify registry 2
        Authorization.ZkAuthorizationData memory authData2 = auth.getZkAuthorizationData(registryId2);
        assertEq(authData2.vk, vk2, "Verification key for registry 2 should be stored correctly");
        assertEq(authData2.allowedExecutionAddresses.length, 1, "Registry 2 should have one authorized user");
        assertEq(authData2.allowedExecutionAddresses[0], address(0), "Registry 2 should be permissionless");
        assertEq(authData2.route, route, "Registry 2 should have the correct route");
        assertEq(
            auth.zkAuthorizationLastExecutionBlock(registryId2), 0, "Last execution block should be zero for registry"
        );

        vm.stopPrank();
    }

    /**
     * @notice Test removing registries
     */
    function testRemoveRegistries() public {
        vm.startPrank(owner);

        // First add registries
        uint64[] memory registriesToAdd = new uint64[](2);
        registriesToAdd[0] = registryId1;
        registriesToAdd[1] = registryId2;

        address[][] memory users = new address[][](2);
        users[0] = new address[](1);
        users[0][0] = user1;
        users[1] = new address[](1);
        users[1][0] = user2;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](2);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });
        zkAuthData[1] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[1],
            route: route,
            vk: vk2,
            validateBlockNumberExecution: validateBlockNumber2,
            metadataHash: bytes32(0)
        });

        auth.addRegistries(registriesToAdd, zkAuthData);

        // Now remove one registry
        uint64[] memory registriesToRemove = new uint64[](1);
        registriesToRemove[0] = registryId1;

        auth.removeRegistries(registriesToRemove);

        // Verify the authorization data was removed
        Authorization.ZkAuthorizationData memory authData1 = auth.getZkAuthorizationData(registryId1);
        assertEq(authData1.allowedExecutionAddresses.length, 0, "Registry 1 should have no authorized users");

        // Verify last execution block was cleared
        uint64 lastExecBlock = auth.zkAuthorizationLastExecutionBlock(registryId1);
        assertEq(lastExecBlock, 0, "Last execution block should be cleared for registry 1");

        // Verify other registry still exists
        Authorization.ZkAuthorizationData memory authData2 = auth.getZkAuthorizationData(registryId2);
        assertEq(authData2.allowedExecutionAddresses.length, 1, "Registry 2 should still have one authorized user");

        vm.stopPrank();
    }

    /**
     * @notice Test that owner can update a ZkAuthorization route
     */
    function testUpdateroute() public {
        vm.startPrank(owner);

        // Add a registry first
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        auth.addRegistries(registries, zkAuthData);

        // Update the route
        string memory newRoute = "newRoute";
        auth.updateRegistryRoute(registryId1, newRoute);

        // Verify the route was updated
        Authorization.ZkAuthorizationData memory updatedAuthData = auth.getZkAuthorizationData(registryId1);
        assertEq(updatedAuthData.route, newRoute, "Route should be updated correctly");

        vm.stopPrank();

        // Verify that unauthorized users cannot update the route
        vm.startPrank(unauthorized);
        vm.expectRevert();
        auth.updateRegistryRoute(registryId1, "anotherRoute");
        vm.stopPrank();
    }

    /**
     * @notice Test that only owner can add registries
     */
    function test_RevertWhen_AddRegistriesUnauthorized() public {
        vm.startPrank(unauthorized);

        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        vm.expectRevert();
        auth.addRegistries(registries, zkAuthData);

        vm.stopPrank();
    }

    /**
     * @notice Test that only owner can remove registries
     */
    function test_RevertWhen_RemoveRegistriesUnauthorized() public {
        vm.startPrank(unauthorized);

        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        vm.expectRevert();
        auth.removeRegistries(registries);

        vm.stopPrank();
    }

    /**
     * @notice Test handling of invalid registry data
     */
    function test_RevertWhen_AddingRegistriesWithInvalidArrayLengths() public {
        vm.startPrank(owner);

        // Create mismatched arrays
        uint64[] memory registries = new uint64[](2);
        registries[0] = registryId1;
        registries[1] = registryId2;

        address[][] memory users = new address[][](2);
        users[0] = new address[](1);
        users[0][0] = user1;
        users[1] = new address[](1);
        users[1][0] = user2;

        // Only one entry, should have two
        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        vm.expectRevert("Array lengths must match");
        auth.addRegistries(registries, zkAuthData);

        vm.stopPrank();
    }

    function test_RevertWhen_AddRegistriesNoVerificationRouter() public {
        vm.startPrank(owner);

        // Set verification router to zero address
        auth.setVerificationRouter(address(0));

        // Create registry data
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        // Should fail because verification router is not set
        vm.expectRevert("Verification router not set");
        auth.addRegistries(registries, zkAuthData);

        vm.stopPrank();
    }

    // ======================= ZK MESSAGE EXECUTION TESTS =======================

    function test_RevertWhen_InvalidAuthorizationContract() public {
        vm.startPrank(owner);

        // Add registry with only user1 authorized
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        auth.addRegistries(registries, zkAuthData);

        // Create a ZK message with an invalid authorization contract
        bytes memory zkMessage = createDummyZKMessage(registryId1, address(user1));
        bytes memory dummyProof = hex"deadbeef"; // Dummy proof data

        // Should fail because address is not the authorization contract
        vm.expectRevert("Invalid authorization contract");
        auth.executeZKMessage(zkMessage, dummyProof, dummyProof);

        vm.stopPrank();
    }

    /**
     * @notice Test unauthorized address verification for ZK message execution
     */
    function test_RevertWhen_ExecuteZKMessageUnauthorized() public {
        vm.startPrank(owner);

        // Add registry with only user1 authorized
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        Authorization.ZkAuthorizationData[] memory zkAuthData = new Authorization.ZkAuthorizationData[](1);
        zkAuthData[0] = Authorization.ZkAuthorizationData({
            allowedExecutionAddresses: users[0],
            route: route,
            vk: vk1,
            validateBlockNumberExecution: validateBlockNumber1,
            metadataHash: bytes32(0)
        });

        auth.addRegistries(registries, zkAuthData);

        vm.stopPrank();

        // Try to execute ZK message from unauthorized address
        vm.startPrank(unauthorized);

        // Create a ZK message
        bytes memory zkMessage = createDummyZKMessage(registryId1, address(auth));
        bytes memory dummyProof = hex"deadbeef"; // Dummy proof data

        // Should fail because address is unauthorized
        vm.expectRevert("Unauthorized address for this registry");
        auth.executeZKMessage(zkMessage, dummyProof, dummyProof);

        vm.stopPrank();
    }

    // ======================= HELPER FUNCTIONS =======================

    /**
     * @notice Create a dummy ZK message for testing
     * @param _registryId Registry ID to include in the message
     * @param _authorizationContract Address of the authorization contract
     * @return Encoded ZK message bytes
     */
    function createDummyZKMessage(uint64 _registryId, address _authorizationContract)
        internal
        view
        returns (bytes memory)
    {
        return createDummyZKMessageWithBlockNumber(_registryId, uint64(block.number + 1), _authorizationContract);
    }

    /**
     * @notice Create a dummy ZK message with a specific block number
     * @param _registryId Registry ID to include in the message
     * @param _blockNumber Block number to include in the message
     * @return Encoded ZK message bytes
     */
    function createDummyZKMessageWithBlockNumber(
        uint64 _registryId,
        uint64 _blockNumber,
        address _authorizationContract
    ) internal view returns (bytes memory) {
        // Create a simple processor message (Pause message)
        IProcessorMessageTypes.ProcessorMessage memory processorMessage = IProcessorMessageTypes.ProcessorMessage({
            messageType: IProcessorMessageTypes.ProcessorMessageType.Pause,
            message: bytes("")
        });

        // Create the ZK message
        Authorization.ZKMessage memory zkMessage = Authorization.ZKMessage({
            registry: _registryId,
            blockNumber: _blockNumber,
            authorizationContract: _authorizationContract,
            processorMessage: processorMessage
        });

        bytes memory rootBytes = abi.encodePacked(coprocessorRoot);
        return bytes.concat(rootBytes, abi.encode(zkMessage));
    }
}
