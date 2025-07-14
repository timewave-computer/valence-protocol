// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IERC721Receiver} from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";

/**
 * @title Account
 * @dev Abstract contract that manages approved libraries with ownership control. Any account contract will inherit from this.
 * Inherits from OpenZeppelin's Ownable for basic access control
 * and implements the IERC721Receiver interface to handle ERC721 token transfers.
 */
abstract contract Account is Ownable, IERC721Receiver {
    /// @notice Mapping to track approved library addresses
    /// @dev Maps library address to approval status (true = approved, false = not approved)
    mapping(address => bool) public approvedLibraries;

    /// @dev Emitted when trying to execute through an address that is not the owner or an approved library
    error NotOwnerOrLibrary(address _sender);

    /// @dev Emitted when trying to execute a call with a zero address target
    error ZeroAddressTarget();

    /**
     * @dev Contract constructor
     * @param _owner Address that will be set as the initial owner
     * @param _libraries Array of initial library addresses to approve
     */
    constructor(address _owner, address[] memory _libraries) Ownable(_owner) {
        for (uint8 i = 0; i < _libraries.length; i++) {
            approvedLibraries[_libraries[i]] = true;
        }
    }

    /**
     * @dev Approves a new library
     * @param _library Address of the library to approve
     * @notice Can only be called by the contract owner
     */
    function approveLibrary(address _library) external onlyOwner {
        approvedLibraries[_library] = true;
    }

    /**
     * @dev Removes approval for a library
     * @param _library Address of the library to remove
     * @notice Can only be called by the contract owner
     */
    function removeLibrary(address _library) external onlyOwner {
        delete approvedLibraries[_library];
    }

    /**
     * @dev Executes encoded call data sent by an approved library
     * @param _target Address of the contract to call
     * @param _value Amount of native tokens to send with the call
     * @param _data Encoded function call data
     * @return result Bytes returned from the call
     * @notice Only calls from approved libraries or owner are allowed
     */
    function execute(address _target, uint256 _value, bytes calldata _data) external returns (bytes memory result) {
        if (!approvedLibraries[msg.sender] && msg.sender != owner()) {
            revert NotOwnerOrLibrary(msg.sender);
        }

        // Ensure the target address is not zero
        // This prevents accidental calls to the zero address which might lead to loss of funds or unexpected behavior
        if (_target == address(0)) revert ZeroAddressTarget();

        (bool success, bytes memory returnData) = _target.call{value: _value}(_data);

        if (!success) {
            if (returnData.length > 0) {
                assembly {
                    // returnData contains ABI-encoded error data:
                    // - First 32 bytes: Length of the error data (accessed via mload(returnData))
                    // - Next n bytes: Actual error data

                    // add(32, returnData): Skip first 32 bytes (length) to get to actual error data
                    // mload(returnData): Load the length of the error data
                    // revert(error_data_pointer, error_data_length)
                    revert(add(32, returnData), mload(returnData))
                }
            } else {
                revert("Contract call failed without error data");
            }
        }

        return returnData;
    }

    /**
     * @dev Implementation of the {IERC721Receiver} interface.
     * Accepts all token transfers.
     */
    function onERC721Received(address, address, uint256, bytes calldata) external pure override returns (bytes4) {
        return this.onERC721Received.selector;
    }

    /// @dev Allows the contract to receive native tokens (e.g. ETH) that can later be used by approved libraries or the owner in execute() calls
    receive() external payable {}
}
