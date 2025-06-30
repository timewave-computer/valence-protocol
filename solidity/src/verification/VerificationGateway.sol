// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {UUPSUpgradeable} from "@openzeppelin-contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";

/**
 * @title VerificationGateway
 * @dev Abstract contract that serves as a base for verification gateways.
 * This contract provides the foundation for verifying proofs against registered verification keys.
 */
abstract contract VerificationGateway is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    /// @notice Generic verifier address that will be specialized in derived contracts
    address public verifier;

    /**
     * @notice Mapping of program verification keys by user address and registry ID
     * @dev Maps: user address => registry ID => verification key
     */
    mapping(address => mapping(uint64 => bytes32)) public programVKs;

    // Storage gap - reserves slots for future versions
    uint256[50] private __gap;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /**
     * @notice Initializes the verification gateway replacing the constructor with an inizializer with the verifier address
     * @param _verifier Address of the verification contract
     */
    function initialize(address _verifier) external initializer {
        __Ownable_init(msg.sender);
        __UUPSUpgradeable_init();
        require(_verifier != address(0), "Verifier cannot be zero address");
        verifier = _verifier;
    }

    /**
     * @notice Updates the verifier address
     * @dev Only the owner can update the verifier address
     * @param _verifier The new verifier address
     */
    function updateVerifier(address _verifier) external onlyOwner {
        require(_verifier != address(0), "Verifier cannot be zero address");
        verifier = _verifier;
    }

    /**
     * @notice Adds a verification key for a specific registry ID
     * @dev Only the sender can add a VK for their own address
     * @param registry The registry ID to associate with the verification key
     * @param vk The verification key to register
     */
    function addRegistry(uint64 registry, bytes32 vk) external {
        programVKs[msg.sender][registry] = vk;
    }

    /**
     * @notice Removes a verification key for a specific registry ID
     * @dev Only the sender can remove a VK for their own address
     * @param registry The registry ID to remove
     */
    function removeRegistry(uint64 registry) external {
        delete programVKs[msg.sender][registry];
    }

    /**
     * @notice Abstract verification function to be implemented by derived contracts
     * @dev Different verification gateways will implement their own verification logic
     * @param registry The registry data used in verification
     * @param proof The proof to verify
     * @param message The message associated with the proof
     * @return True if the proof is valid, false or revert otherwise
     */
    function verify(uint64 registry, bytes calldata proof, bytes calldata message) external virtual returns (bool);
}
