// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IDynamicRatioOracle} from "../../src/libraries/interfaces/splitter/IDynamicRatioOracle.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

contract MockDynamicRatioOracle is IDynamicRatioOracle {
    mapping(IERC20 => uint256) public tokenRatios;

    function setRatio(IERC20 token, uint256 ratio) external {
        tokenRatios[token] = ratio;
    }

    function queryDynamicRatio(IERC20 token, bytes calldata /*params*/ ) external view returns (uint256) {
        uint256 ratio = tokenRatios[token];
        return ratio > 0 ? ratio : 300_000_000_000_000_000; // Default 30% if not set
    }
}
