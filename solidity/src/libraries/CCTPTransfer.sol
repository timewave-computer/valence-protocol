// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {ITokenMessenger} from "./interfaces/ITokenMessenger.sol";

/**
 * @title CCTPTransfer
 * @dev Contract for automatically transferring tokens using the CCTP protocol
 */
contract CCTPTransfer is Library {
    struct CCTPTransferConfig {
        Account inputAccount;
        ITokenMessenger cctpTokenMessenger;
        bytes32 mintRecipient;
        address burnToken;
        uint32 destinationDomain;
    }

    CCTPTransferConfig public config;

    constructor(
        address _owner,
        address _processor,
        bytes memory _config
    ) Library(_owner, _processor, _config) {}

    /**
     * @dev Validates configuration, checking for zero addresses
     * @param _config Raw configuration bytes
     * @return Decoded and validated config
     */
    function validateConfig(
        bytes memory _config
    ) internal pure returns (CCTPTransferConfig memory) {
        CCTPTransferConfig memory decodedConfig = abi.decode(
            _config,
            (CCTPTransferConfig)
        );

        if (decodedConfig.cctpTokenMessenger == ITokenMessenger(address(0))) {
            revert("CCTP Token Messenger can't be zero address");
        }

        if (decodedConfig.burnToken == address(0)) {
            revert("Burn token can't be zero address");
        }
        return decodedConfig;
    }

    /**
     * @dev Updates CCTPTransfer configuration
     * @param _config New configuration
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    function transfer() public {
    }
}
