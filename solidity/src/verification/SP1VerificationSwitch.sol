// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Verifier} from "./interfaces/Verifier.sol";
import {ISP1Verifier} from "succinctlabs-sp1-contracts/src/ISP1Verifier.sol";

/**
 * @title SP1VerificationSwitch
 * @notice Switch contract for verifying SP1 zero-knowledge proofs with domain separation
 * @dev This contract acts as a wrapper around the SP1 verifier, providing dual verification
 *      for both program proofs and domain-specific proofs. It implements the Verifier interface
 *      and delegates actual verification to the SP1Verifier contract.
 */
contract SP1VerificationSwitch is Verifier {
    /// @notice Address of the SP1 verifier contract
    /// @dev This should be a valid SP1Verifier contract address
    address public sp1Verifier;

    /// @notice Domain verification key used for domain-specific proof verification
    /// @dev This is a 32-byte verification key for the domain circuit
    bytes32 public domainVK;

    /**
     * @notice Returns the verifier cast to the ISP1Verifier interface
     * @return The verifier as an ISP1Verifier interface
     * @dev This is a convenience function for type casting
     */
    function getVerifier() public view returns (ISP1Verifier) {
        return ISP1Verifier(sp1Verifier);
    }

    /**
     * @notice Constructs the SP1VerificationSwitch
     * @param _sp1Verifier Address of the SP1 verifier contract
     * @param _domainVK Domain verification key (32 bytes)
     * @dev The SP1 verifier address must not be the zero address
     */
    constructor(address _sp1Verifier, bytes32 _domainVK) {
        // Check that verifier is a valid address
        require(_sp1Verifier != address(0), "Invalid SP1 verifier address");

        sp1Verifier = _sp1Verifier;
        domainVK = _domainVK;
    }

    /**
     * @notice Verifies both program and domain proofs using the SP1 verifier
     * @param vk The verification key for the program proof (must be 32 bytes because SP1 uses a 32-byte hash)
     * @param proof The program proof to verify
     * @param inputs The public inputs for the program proof
     * @param payload The domain proof payload
     * @return bool True if both proofs are valid, reverts otherwise
     * @dev This function performs dual verification:
     *      1. Verifies the program proof using the provided vk
     *      2. Verifies the domain proof using the stored domainVK and first 32 bytes of inputs
     * @custom:verification The function uses the first 32 bytes of inputs as the coprocessor root for domain verification
     */
    function verify(bytes calldata vk, bytes calldata proof, bytes calldata inputs, bytes calldata payload)
        external
        view
        override
        returns (bool)
    {
        // Validation
        require(vk.length == 32, "VK must be 32 bytes");

        // Convert to bytes32 - we've already checked the length above so it won't truncate
        bytes32 vkBytes32 = bytes32(vk);

        // Get verifier
        ISP1Verifier _sp1Verifier = getVerifier();

        // Verify program proofs
        _sp1Verifier.verifyProof(vkBytes32, inputs, proof);

        // Build the domain inputs by getting the first 32 bytes of the inputs (coprocessor root)
        bytes memory domainInputs = bytes(inputs[0:32]);

        // Verify the domain proof
        _sp1Verifier.verifyProof(domainVK, domainInputs, payload);

        return true;
    }
}
