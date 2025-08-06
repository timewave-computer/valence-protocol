// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title Verifier
 * @dev Common interface for a verifier contract that can verify zk proofs.
 *
 * This interface is used to abstract the verification process, allowing different implementations
 * to be used without changing the code that relies on it.
 */
interface Verifier {
    /**
     * @dev Verifies a proof against the given inputs with a given VK.
     * @param vk The verification key to use for the proof.
     * @param proof The proof to verify.
     * @param inputs The inputs to verify against.
     * @param payload Additional payload, which in first version will be the domain proof to be validated against the domain VK.
     * @return True if the proof is valid, false otherwise.
     */
    function verify(bytes calldata vk, bytes calldata proof, bytes calldata inputs, bytes calldata payload)
        external
        returns (bool);
}
