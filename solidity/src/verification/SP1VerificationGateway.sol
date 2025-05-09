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

    /**
     * @notice Initializes the SP1 verification gateway
     * @param _coprocessorRoot The root hash of the coprocessor
     * @param _verifier Address of the SP1 verifier contract
     */
    constructor(bytes32 _coprocessorRoot, address _verifier) VerificationGateway(_coprocessorRoot, _verifier) {}

    /**
     * @notice Verifies a proof using the SP1 verifier
     * @param registry The registry used in verification
     * @param proof The proof to verify
     * @param message The message associated with the proof
     */
    function verify(uint64 registry, bytes calldata proof, bytes calldata message)
        external
        view
        override
        returns (bool)
    {
        // Get the VK for the sender and the registry
        bytes32 vk = programVKs[msg.sender][registry];

        // If the VK is not set, revert
        require(vk != bytes32(0), "VK not set for sender and registry");

        // Call the specific verifier
        ISP1Verifier sp1Verifier = getVerifier();

        sp1Verifier.verifyProof(vk, proof, message);

        return true;
    }
}
