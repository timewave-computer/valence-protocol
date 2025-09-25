// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

// interface for operator functionality inspired by ERC-6909, as described in ERC-7540:
// https://eips.ethereum.org/EIPS/eip-7540#operators

interface IERC7540Operator {
    /**
     * @dev Emitted when `controller` grants or revokes operator status for a `spender`.
     */
    event OperatorSet(
        address indexed controller,
        address indexed operator,
        bool approved
    );

    /**
     * @dev Returns true if `operator` is set as an operator for `controller`.
     */
    function isOperator(
        address controller,
        address operator
    ) external view returns (bool);

    /**
     * @dev Grants or revokes `operator` rights to issue ERC-7540 requests on behalf of the caller (controller).
     *
     * Returns true if the operation was successful.
     */
    function setOperator(
        address operator,
        bool approved
    ) external returns (bool);
}
