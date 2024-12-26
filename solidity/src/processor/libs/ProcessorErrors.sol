// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

library ProcessorErrors {
    error UnauthorizedAccess();
    error NotAuthorizationContract();
    error InvalidAddress();
    error ProcessorPaused();
    error UnsupportedOperation();
    error InvalidOriginDomain();
}
