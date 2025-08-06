// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Verifier} from "./interfaces/Verifier.sol";

/**
 * @title VerificationRouter
 * @notice A routing contract that manages multiple verifiers
 * @dev Immutable append-only contract that allows the owner to add verification routes with their corresponding verifier address
 *      and allows routing of verification requests to the appropriate verifier based on a name identifier.
 */
contract VerificationRouter is Ownable {
    /// @notice Mapping of route names to their corresponding verifier contract addresses
    /// @dev Maps string identifiers to contract addresses for verification routing
    mapping(string => address) public routes;

    /**
     * @notice Emitted when a new verification route is added
     * @param name The identifier name for the route
     * @param route The address of the verifier contract
     */
    event RouteAdded(string indexed name, address indexed route);

    /**
     * @notice Contract constructor that sets the deployer as the owner
     * @dev Inherits from Ownable and sets msg.sender as the initial owner
     */
    constructor() Ownable(msg.sender) {}

    /**
     * @notice Adds a new verification route
     * @dev Only callable by the contract owner. Prevents overwriting existing routes.
     * @param name The identifier name for the verification route
     * @param route The address of the verifier contract to route to
     * @custom:requirements
     * - Caller must be the contract owner
     * - Route address cannot be the zero address
     * - Route name must not already exist
     */
    function addRoute(string memory name, address route) external onlyOwner {
        require(route != address(0), "Invalid route address");
        require(routes[name] == address(0), "Route already exists");
        routes[name] = route;

        emit RouteAdded(name, route);
    }

    /**
     * @notice Retrieves the verifier contract address for a given route name
     * @param name The identifier name of the route to look up
     * @return The address of the verifier contract, or address(0) if route doesn't exist
     */
    function getRoute(string memory name) external view returns (address) {
        return routes[name];
    }

    /**
     * @notice Performs verification by routing to the appropriate verifier contract
     * @dev Routes the verification request to the verifier contract registered under the given name.
     *      The function will revert if the route doesn't exist or if the underlying verification fails.
     * @param name The identifier name of the verification route to use
     * @param vk The verification key data required by the verifier
     * @param proof The cryptographic proof to be verified
     * @param inputs The public inputs associated with the proof
     * @param payload Additional data payload required by the verifier, for example a domain proof that needs to be verified against a domain VK
     * @return Always returns true if verification succeeds (reverts on failure)
     * @custom:requirements
     * - The specified route name must exist in the routes mapping
     * - The target verifier contract must implement the Verifier interface
     * - All verification parameters must be valid according to the target verifier
     */
    function verify(
        string memory name,
        bytes calldata vk,
        bytes calldata proof,
        bytes calldata inputs,
        bytes calldata payload
    ) external returns (bool) {
        // Ensure the route exists
        address route = routes[name];
        require(route != address(0), "Route not found");

        // Call the verify function on the Verifier contract at the route address
        Verifier(route).verify(vk, proof, inputs, payload);

        // Return true if the verification was successful
        return true;
    }
}
