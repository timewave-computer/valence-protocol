// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VerificationGateway} from "./VerificationGateway.sol";
import {ISP1Verifier} from "succinctlabs-sp1-contracts/src/ISP1Verifier.sol";

/**
 * @title SP1VerificationGateway
 * @dev Specific implementation of VerificationGateway for the SP1 verifier
 */
contract SP1VerificationGateway is VerificationGateway {
    /**
     * @notice Returns the verifier cast to the ISP1Verifier interface
     * @return The verifier as an ISP1Verifier
     */
    function getVerifier() public view returns (ISP1Verifier) {
        return ISP1Verifier(verifier);
    }

    constructor() VerificationGateway() {}

    /**
     * @dev Function that should revert when `msg.sender` is not authorized to upgrade the contract. Called by
     * {upgradeTo} and {upgradeToAndCall}.
     *
     * Normally, this function will use an xref:access.adoc[access control] modifier such as {Ownable-onlyOwner}.
     *
     * @param newImplementation address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        // Upgrade logic comes here
    }

    /**
     * @notice Verifies a proof using the SP1 verifier
     * @param registry The registry used in verification
     * @param proof The proof to verify
     * @param message The message associated with the proof
     * @param domainProof The domain proof to verify
     * @param domainMessage The domain message associated with the domain proof
     */
    function verify(
        uint64 registry,
        bytes calldata proof,
        bytes calldata message,
        bytes calldata domainProof,
        bytes calldata domainMessage
    ) external view override returns (bool) {
        // Get the VK for the sender and the registry
        bytes memory vk = programVKs[msg.sender][registry];

        // If the VK is not set, revert
        require(vk.length != 0, "VK not set for user and registry");
        require(vk.length == 32, "VK must be 32 bytes");

        // Call the specific verifier
        ISP1Verifier sp1Verifier = getVerifier();

        // Get the domainVK
        bytes memory _domainVK = domainVK;

        // Convert bytes to bytes32
        bytes32 vkBytes32;
        assembly {
            // Skips the first 32 bytes (length) and reads the next 32 bytes
            // This assumes vk is at least 32 bytes long, which is checked above
            vkBytes32 := mload(add(vk, 32))
        }

        bytes32 domainVKBytes32;
        require(_domainVK.length == 32, "Domain VK must be 32 bytes");
        assembly {
            // Skips the first 32 bytes (length) and reads the next 32 bytes
            // This assumes vk is at least 32 bytes long, which is checked above
            domainVKBytes32 := mload(add(_domainVK, 32))
        }

        sp1Verifier.verifyProof(vkBytes32, message, proof);
        sp1Verifier.verifyProof(domainVKBytes32, domainMessage, domainProof);

        return true;
    }
}
