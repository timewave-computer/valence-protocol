// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Account} from "./Account.sol";

/**
 * @title BaseAccount
 * @dev Basic implementation of Account contract with no additional functionality
 */
contract BaseAccount is Account {
    constructor(address _owner, address[] memory _libraries) Account(_owner, _libraries) {}
}
