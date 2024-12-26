// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

library ProcessorErrors {
    error UnauthorizedAccessError();
    error NotAuthorizationContractError();
    error InvalidAddressError();
    error ProcessorPausedError();
    error UnsupportedOperationError();
}
