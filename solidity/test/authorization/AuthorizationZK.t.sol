// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {Authorization} from "../../src/authorization/Authorization.sol";
import {SP1VerificationGateway} from "../../src/verification/SP1VerificationGateway.sol";
import {ProcessorBase} from "../../src/processor/ProcessorBase.sol";
import {LiteProcessor} from "../../src/processor/LiteProcessor.sol";
import {IProcessorMessageTypes} from "../../src/processor/interfaces/IProcessorMessageTypes.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

/**
 * @title AuthorizationZKTest
 * @notice Test suite for the ZK authorization flow in the Authorization contract
 * @dev Tests focus on registry management and unauthorized access verification
 */
contract AuthorizationZKTest is Test {
    Authorization auth;
    LiteProcessor processor;
    SP1VerificationGateway verificationGateway;

    address owner = address(0x1);
    address user1 = address(0x2);
    address user2 = address(0x3);
    address unauthorized = address(0x4);
    address verifier = address(0x5);
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

        // Deploy verification gateway
        verificationGateway = new SP1VerificationGateway();

        bytes memory domainVK = abi.encodePacked(bytes32(uint256(0xdeadbeef)));
        bytes memory initializeData =
            abi.encodeWithSelector(verificationGateway.initialize.selector, verifier, domainVK);

        // Deploy the proxy and initialize it
        ERC1967Proxy proxy = new ERC1967Proxy(address(verificationGateway), initializeData);
        verificationGateway = SP1VerificationGateway(address(proxy));

        // Deploy authorization contract with verification gateway
        auth = new Authorization(owner, address(processor), address(verificationGateway), true);

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

        // Create verification keys
        bytes[] memory vks = new bytes[](2);
        vks[0] = vk1;
        vks[1] = vk2;

        // Set block number validation flags
        bool[] memory validateBlockNumbers = new bool[](2);
        validateBlockNumbers[0] = validateBlockNumber1;
        validateBlockNumbers[1] = validateBlockNumber2;

        // Add registries
        auth.addRegistries(registries, users, vks, validateBlockNumbers);

        // Verify registry 1
        bytes memory storedVk1 = verificationGateway.programVKs(address(auth), registryId1);
        assertEq(storedVk1, vk1, "Verification key for registry 1 should be stored correctly");

        // Check authorized users for registry 1
        address[] memory authorizedUsers1 = auth.getZkAuthorizationsList(registryId1);
        assertEq(authorizedUsers1.length, 2, "Registry 1 should have two authorized users");
        assertEq(authorizedUsers1[0], user1, "Registry 1 should authorize user1");
        assertEq(authorizedUsers1[1], user2, "Registry 1 should authorize user2");

        // Verify registry 2
        bytes memory storedVk2 = verificationGateway.programVKs(address(auth), registryId2);
        assertEq(storedVk2, vk2, "Verification key for registry 2 should be stored correctly");

        // Check authorized users for registry 2
        address[] memory authorizedUsers2 = auth.getZkAuthorizationsList(registryId2);
        assertEq(authorizedUsers2.length, 1, "Registry 2 should have one authorized user");
        assertEq(authorizedUsers2[0], address(0), "Registry 2 should be permissionless");

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

        bytes[] memory vks = new bytes[](2);
        vks[0] = vk1;
        vks[1] = vk2;

        bool[] memory validateBlockNumbers = new bool[](2);
        validateBlockNumbers[0] = validateBlockNumber1;
        validateBlockNumbers[1] = validateBlockNumber2;

        auth.addRegistries(registriesToAdd, users, vks, validateBlockNumbers);

        // Verify registries were added
        bytes memory storedVk1 = verificationGateway.programVKs(address(auth), registryId1);
        assertEq(storedVk1, vk1, "Registry 1 should be added");

        // Now remove one registry
        uint64[] memory registriesToRemove = new uint64[](1);
        registriesToRemove[0] = registryId1;

        auth.removeRegistries(registriesToRemove);

        // Verify registry was removed
        bytes memory removedVk = verificationGateway.programVKs(address(auth), registryId1);
        assertEq(removedVk.length, 0, "Registry 1 should be removed from verification gateway");

        // Verify the authorization data was also removed
        address[] memory authorizedUsers1 = auth.getZkAuthorizationsList(registryId1);
        assertEq(authorizedUsers1.length, 0, "Registry 1 should have no authorized users");

        // Verify last execution block was cleared
        uint64 lastExecBlock = auth.zkAuthorizationLastExecutionBlock(registryId1);
        assertEq(lastExecBlock, 0, "Last execution block should be cleared for registry 1");

        // Verify other registry still exists
        bytes memory storedVk2 = verificationGateway.programVKs(address(auth), registryId2);
        assertEq(storedVk2, vk2, "Registry 2 should still exist");

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

        bytes[] memory vks = new bytes[](1);
        vks[0] = vk1;

        bool[] memory validateBlockNumbers = new bool[](2);
        validateBlockNumbers[0] = validateBlockNumber1;
        validateBlockNumbers[1] = validateBlockNumber2;

        vm.expectRevert();
        auth.addRegistries(registries, users, vks, validateBlockNumbers);

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

        address[][] memory users = new address[][](1); // Only one entry, should have two
        users[0] = new address[](1);
        users[0][0] = user1;

        bytes[] memory vks = new bytes[](2);
        vks[0] = vk1;
        vks[1] = vk2;

        bool[] memory validateBlockNumbers = new bool[](2);
        validateBlockNumbers[0] = validateBlockNumber1;
        validateBlockNumbers[1] = validateBlockNumber2;

        vm.expectRevert("Array lengths must match");
        auth.addRegistries(registries, users, vks, validateBlockNumbers);

        vm.stopPrank();
    }

    function test_RevertWhen_AddRegistriesNoVerificationGateway() public {
        vm.startPrank(owner);

        // Deploy a new authorization contract without a verification gateway
        Authorization authWithoutGateway = new Authorization(owner, address(processor), address(0), true);

        // Create registry data
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        bytes[] memory vks = new bytes[](1);
        vks[0] = vk1;

        bool[] memory validateBlockNumbers = new bool[](1);
        validateBlockNumbers[0] = validateBlockNumber1;

        // Should fail because verification gateway is not set
        vm.expectRevert("Verification gateway not set");
        authWithoutGateway.addRegistries(registries, users, vks, validateBlockNumbers);

        vm.stopPrank();
    }

    function test_RevertWhen_RemoveRegistriesNoVerificationGateway() public {
        vm.startPrank(owner);

        // Deploy a new authorization contract without a verification gateway
        Authorization authWithoutGateway = new Authorization(owner, address(processor), address(0), true);

        // Should fail because verification gateway is not set
        vm.expectRevert("Verification gateway not set");
        authWithoutGateway.removeRegistries(new uint64[](1));

        vm.stopPrank();
    }

    // ======================= ZK MESSAGE EXECUTION TESTS =======================

    function test_RevertWhen_VerificationGatewayNotSet() public {
        vm.startPrank(owner);

        // Deploy a new authorization contract without a verification gateway
        Authorization authWithoutGateway = new Authorization(owner, address(processor), address(0), true);

        // Create a ZK message
        bytes memory zkMessage = createDummyZKMessage(registryId1, address(auth));
        bytes memory dummyProof = hex"deadbeef"; // Dummy proof data

        // Should fail because verification gateway is not set
        vm.expectRevert("Verification gateway not set");
        authWithoutGateway.executeZKMessage(zkMessage, dummyProof, zkMessage, dummyProof);

        vm.stopPrank();
    }

    function test_RevertWhen_InvalidAuthorizationContract() public {
        vm.startPrank(owner);

        // Add registry with only user1 authorized
        uint64[] memory registries = new uint64[](1);
        registries[0] = registryId1;

        address[][] memory users = new address[][](1);
        users[0] = new address[](1);
        users[0][0] = user1;

        bytes[] memory vks = new bytes[](1);
        vks[0] = vk1;

        bool[] memory validateBlockNumbers = new bool[](1);
        validateBlockNumbers[0] = validateBlockNumber1;

        auth.addRegistries(registries, users, vks, validateBlockNumbers);

        // Create a ZK message with an invalid authorization contract
        bytes memory zkMessage = createDummyZKMessage(registryId1, address(user1));
        bytes memory dummyProof = hex"deadbeef"; // Dummy proof data

        // Should fail because address is not the authorization contract
        vm.expectRevert("Invalid authorization contract");
        auth.executeZKMessage(zkMessage, dummyProof, zkMessage, dummyProof);

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

        bytes[] memory vks = new bytes[](1);
        vks[0] = vk1;

        bool[] memory validateBlockNumbers = new bool[](1);
        validateBlockNumbers[0] = validateBlockNumber1;

        auth.addRegistries(registries, users, vks, validateBlockNumbers);

        vm.stopPrank();

        // Try to execute ZK message from unauthorized address
        vm.startPrank(unauthorized);

        // Create a ZK message
        bytes memory zkMessage = createDummyZKMessage(registryId1, address(auth));
        bytes memory dummyProof = hex"deadbeef"; // Dummy proof data

        // Should fail because address is unauthorized
        vm.expectRevert("Unauthorized address for this registry");
        auth.executeZKMessage(zkMessage, dummyProof, zkMessage, dummyProof);

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
