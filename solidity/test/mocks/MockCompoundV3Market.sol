// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

contract MockCompoundV3Market {
    address public baseToken;

    constructor(address _baseToken) {
        baseToken = _baseToken;
    }
}
